-- Per-container fixed-interval rate limiter for Fluent Bit.
--
-- Keyed on the Docker-daemon-set `container_name` record field (fallback:
-- fluentd tag). Override map is parsed with a strict delimiter split — never
-- load / loadstring / dofile on env content.

local DEFAULT_CAP = 100
local DEFAULT_INTERVAL = 1
local DEFAULT_MAX_KEYS = 256

local getenv = os.getenv
local printer = print

local default_cap = DEFAULT_CAP
local interval = DEFAULT_INTERVAL
local max_keys = DEFAULT_MAX_KEYS
local overrides = {}
local state = {}

local function trim(s)
  return (s:gsub("^%s+", ""):gsub("%s+$", ""))
end

local function sanitize(s, max_len)
  s = tostring(s or "")
  s = s:gsub("[%c]", "?")
  max_len = max_len or 128
  if #s > max_len then
    s = s:sub(1, max_len) .. "..."
  end
  return s
end

local function parse_non_negative_int(s, fallback)
  if type(s) ~= "string" or not s:match("^%d+$") then
    return fallback
  end
  return tonumber(s, 10) or fallback
end

-- Strict `name=integer` comma-separated parser.
-- Invalid entries are skipped and logged; the filter never crashes on them.
-- Injection-style values (code, decimals, empty names) are rejected.
local function parse_overrides(s)
  local map = {}
  if type(s) ~= "string" or s == "" then
    return map
  end
  for entry in string.gmatch(s, "[^,]+") do
    entry = trim(entry)
    if entry ~= "" then
      local name, val = entry:match("^([^=]+)=([^=]+)$")
      if name then
        name = trim(name)
        val = trim(val)
        if name ~= "" and val:match("^%d+$") then
          local n = tonumber(val, 10)
          if n then
            map[name] = n
          end
        else
          printer(
            "[rate_limit] skip invalid override (need name=integer): "
              .. sanitize(entry)
          )
        end
      else
        printer(
          "[rate_limit] skip invalid override (need name=integer): "
            .. sanitize(entry)
        )
      end
    end
  end
  return map
end

local function reload_config()
  default_cap = parse_non_negative_int(
    getenv("FLUENTBIT_RATE_LIMIT_DEFAULT"),
    DEFAULT_CAP
  )
  interval = parse_non_negative_int(
    getenv("FLUENTBIT_RATE_LIMIT_INTERVAL"),
    DEFAULT_INTERVAL
  )
  if interval < 1 then
    interval = DEFAULT_INTERVAL
  end
  max_keys = parse_non_negative_int(
    getenv("FLUENTBIT_RATE_LIMIT_MAX_KEYS"),
    DEFAULT_MAX_KEYS
  )
  if max_keys < 1 then
    max_keys = DEFAULT_MAX_KEYS
  end
  overrides = parse_overrides(getenv("FLUENTBIT_RATE_LIMIT_OVERRIDES") or "")
end

local function reset_state()
  state = {}
end

local function to_seconds(timestamp)
  if type(timestamp) == "table" then
    return timestamp.sec or timestamp[1] or 0
  elseif type(timestamp) == "number" then
    return timestamp
  end
  return os.time()
end

local function window_id(now)
  return math.floor(now / interval)
end

local function key_count()
  local n = 0
  for _ in pairs(state) do
    n = n + 1
  end
  return n
end

local function evict_lru()
  local oldest_key = nil
  local oldest_seen = nil
  for k, st in pairs(state) do
    if oldest_seen == nil or st.last_seen < oldest_seen then
      oldest_key = k
      oldest_seen = st.last_seen
    end
  end
  if oldest_key ~= nil then
    state[oldest_key] = nil
  end
end

local function ensure_key(key, now)
  local st = state[key]
  if st then
    st.last_seen = now
    return st
  end
  if key_count() >= max_keys then
    evict_lru()
  end
  st = {
    count = 0,
    window = window_id(now),
    notified = false,
    last_seen = now,
  }
  state[key] = st
  return st
end

local function cap_for(name)
  if overrides[name] ~= nil then
    return overrides[name]
  end
  local stripped = name:gsub("^/", "")
  if overrides[stripped] ~= nil then
    return overrides[stripped]
  end
  local slashed = "/" .. stripped
  if overrides[slashed] ~= nil then
    return overrides[slashed]
  end
  return default_cap
end

-- Docker's fluentd extra sets container_name to "/name"; awslogs {{.Name}}
-- is the bare name. Strip the slash for keying AND rewrite the record so
-- cloudwatch log_stream_template $container_name matches existing streams.
local function record_key(tag, record)
  if type(record) == "table" then
    local name = record["container_name"]
    if type(name) == "string" and name ~= "" then
      if name:sub(1, 1) == "/" then
        name = name:sub(2)
        record["container_name"] = name
      end
      if name ~= "" then
        return name
      end
    end
  end
  if type(tag) == "string" and tag ~= "" then
    return tag
  end
  return "unknown"
end

function rate_limit(tag, timestamp, record)
  local now = to_seconds(timestamp)
  local key = record_key(tag, record)
  local cap = cap_for(key)
  local st = ensure_key(key, now)
  local win = window_id(now)

  if st.window ~= win then
    st.window = win
    st.count = 0
    st.notified = false
  end

  if st.count < cap then
    st.count = st.count + 1
    -- 1 = use this record (applies the leading-slash normalization).
    -- 0 would discard record mutations and keep Docker's "/name".
    return 1, timestamp, record
  end

  if not st.notified then
    st.notified = true
    printer(
      string.format(
        "[rate_limit] container %s throttled count=%d cap=%d interval=%ds",
        sanitize(key),
        st.count,
        cap,
        interval
      )
    )
  end

  return -1, timestamp, nil
end

reload_config()

-- Test seam. Fluent Bit only requires the global `rate_limit` callback;
-- this table is unused in production.
RateLimit = {
  parse_overrides = parse_overrides,
  reload_config = reload_config,
  reset = function()
    reset_state()
    reload_config()
  end,
  key_count = key_count,
  has_key = function(k)
    return state[k] ~= nil
  end,
  cap_for = cap_for,
  set_env = function(tbl)
    if tbl == nil then
      getenv = os.getenv
    else
      getenv = function(k)
        return tbl[k]
      end
    end
    reset_state()
    reload_config()
  end,
  set_printer = function(fn)
    printer = fn or print
  end,
  defaults = {
    cap = DEFAULT_CAP,
    interval = DEFAULT_INTERVAL,
    max_keys = DEFAULT_MAX_KEYS,
  },
}
