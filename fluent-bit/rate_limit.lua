-- Per-container fixed-interval rate limiter for Fluent Bit.
--
-- Primary cap is bytes of record["log"] per window (FLUENTBIT_RATE_LIMIT_BYTES
-- / FLUENTBIT_RATE_LIMIT_BYTE_OVERRIDES) so the limit maps to CloudWatch
-- ingest cost. Record count (FLUENTBIT_RATE_LIMIT_DEFAULT / OVERRIDES) is a
-- secondary guard against a flood of tiny lines.
--
-- Defaults are an hourly budget: short windows cannot tell a healthy burst
-- (workloads are bimodal, idle ~0.4 lines/s, bursting to 15k/s) from a loop.
-- Hourly also bounds blast radius and resets 24 times a day.
--
-- Keyed on the Docker-daemon-set `container_name` record field (fallback:
-- fluentd tag). Override maps are parsed with a strict delimiter split —
-- never load / loadstring / dofile on env content.
--
-- Lifetime totals (total_count / total_bytes) accumulate accepted traffic
-- only and survive hourly window reset. They persist until Fluent Bit
-- restarts OR that key is LRU-evicted when FLUENTBIT_RATE_LIMIT_MAX_KEYS
-- (default 256) is hit — eviction deletes the whole per-key entry, including
-- lifetime totals. Do not lift max_keys to retain totals; healthy swarms
-- have far fewer than 256 containers. Totals are snapshotted to
-- /var/log/flb-storage/container_stats.json (storage.path volume) at most
-- once per second. The dump is removed on script load so a volume-persisted
-- file cannot outlive this process.

local DEFAULT_CAP = 500000 -- 500k events / hour
local DEFAULT_BYTE_CAP = 67108864 -- 64 MiB / hour (~1.55 GiB/day)
local DEFAULT_INTERVAL = 3600 -- hourly window
local DEFAULT_MAX_KEYS = 256
local DEFAULT_STATS_PATH = "/var/log/flb-storage/container_stats.json"
local DUMP_INTERVAL = 1 -- at most once per second

local getenv = os.getenv
local printer = print

local default_cap = DEFAULT_CAP
local byte_cap = DEFAULT_BYTE_CAP
local interval = DEFAULT_INTERVAL
local max_keys = DEFAULT_MAX_KEYS
local overrides = {}
local byte_overrides = {}
local state = {}
local stats_path = DEFAULT_STATS_PATH
local last_dump = nil
local dump_dirty = false

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

-- JSON string encoder for dump output. Do not reuse sanitize(): that helper
-- truncates at 128 bytes and is only for [rate_limit] printer lines.
local function json_escape(s)
  s = tostring(s or "")
  s = s:gsub("\\", "\\\\")
  s = s:gsub('"', '\\"')
  s = s:gsub("\b", "\\b")
  s = s:gsub("\f", "\\f")
  s = s:gsub("\n", "\\n")
  s = s:gsub("\r", "\\r")
  s = s:gsub("\t", "\\t")
  s = s:gsub("[%z\1-\31]", function(c)
    return string.format("\\u%04x", string.byte(c))
  end)
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
  byte_cap = parse_non_negative_int(
    getenv("FLUENTBIT_RATE_LIMIT_BYTES"),
    DEFAULT_BYTE_CAP
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
  byte_overrides = parse_overrides(
    getenv("FLUENTBIT_RATE_LIMIT_BYTE_OVERRIDES") or ""
  )
end

local function reset_state()
  state = {}
  last_dump = nil
  dump_dirty = false
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
    bytes = 0,
    total_count = 0,
    total_bytes = 0,
    window = window_id(now),
    notified = false,
    last_seen = now,
  }
  state[key] = st
  return st
end

-- Same lookup shape for record-cap and byte-cap override maps.
local function lookup_override(map, name, fallback)
  if map[name] ~= nil then
    return map[name]
  end
  local stripped = name:gsub("^/", "")
  if map[stripped] ~= nil then
    return map[stripped]
  end
  local slashed = "/" .. stripped
  if map[slashed] ~= nil then
    return map[slashed]
  end
  return fallback
end

local function cap_for(name)
  return lookup_override(overrides, name, default_cap)
end

local function byte_cap_for(name)
  return lookup_override(byte_overrides, name, byte_cap)
end

-- Docker fluentd driver puts the message in `log` (a string). Byte length of
-- that field is the CloudWatch ingest size we are budgeting against.
local function record_log_bytes(record)
  if type(record) ~= "table" then
    return 0
  end
  local log = record["log"]
  if type(log) == "string" then
    return #log
  end
  return 0
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

local function stats_for(key)
  local st = state[key]
  if not st then
    return nil
  end
  return {
    count = st.count,
    bytes = st.bytes,
    total_count = st.total_count or 0,
    total_bytes = st.total_bytes or 0,
    window = st.window,
  }
end

-- Pure encoder: lifetime totals only, never log bodies. No cjson.
local function encode_stats()
  local names = {}
  for k in pairs(state) do
    names[#names + 1] = k
  end
  table.sort(names)
  local parts = {}
  for _, k in ipairs(names) do
    local st = state[k]
    parts[#parts + 1] = string.format(
      '{"container_name":"%s","input_bytes":%d,"input_records":%d}',
      json_escape(k),
      st.total_bytes or 0,
      st.total_count or 0
    )
  end
  return '{"containers":[' .. table.concat(parts, ",") .. "]}"
end

local function write_dump_file(path, json)
  local tmp = path .. ".tmp"
  local f, err = io.open(tmp, "w")
  if not f then
    error(err or "io.open failed")
  end
  local ok, werr = f:write(json)
  f:close()
  if not ok then
    error(werr or "write failed")
  end
  local renamed, rerr = os.rename(tmp, path)
  if not renamed then
    error(rerr or "os.rename failed")
  end
end

-- Throttled atomic dump. I/O is pcall'd so a failure never throws to the
-- caller; in-memory counters stay intact. Single global last_dump (not
-- per-key) so max_keys=256 cannot produce hundreds of writes per second.
-- last_dump advances on any attempt so a missing volume cannot retry I/O
-- on every record; dump_dirty stays set until a write succeeds.
local function dump_stats(path, now)
  if not dump_dirty then
    return false
  end
  now = now or os.time()
  if last_dump ~= nil and (now - last_dump) < DUMP_INTERVAL then
    return false
  end
  path = path or stats_path
  local json = encode_stats()
  -- Advance the throttle before I/O so a throw cannot skip it.
  last_dump = now
  local ok = pcall(write_dump_file, path, json)
  if ok then
    dump_dirty = false
  end
  return ok
end

local function maybe_dump(now)
  pcall(dump_stats, stats_path, now)
end

local function clear_stale_dump(path)
  os.remove(path)
  os.remove(path .. ".tmp")
end

function rate_limit(tag, timestamp, record)
  local now = to_seconds(timestamp)
  local key = record_key(tag, record)
  local cap = cap_for(key)
  local bcap = byte_cap_for(key)
  local st = ensure_key(key, now)
  local win = window_id(now)
  local nbytes = record_log_bytes(record)

  if st.window ~= win then
    st.window = win
    st.count = 0
    st.bytes = 0
    st.notified = false
    -- total_count / total_bytes survive window rollover.
  end

  -- Byte cap is primary (cost). Record cap is a secondary guard.
  local over_records = st.count >= cap
  local over_bytes = (st.bytes + nbytes) > bcap
  if over_records or over_bytes then
    if not st.notified then
      st.notified = true
      printer(
        string.format(
          "[rate_limit] container %s throttled count=%d cap=%d bytes=%d byte_cap=%d interval=%ds",
          sanitize(key),
          st.count,
          cap,
          st.bytes,
          bcap,
          interval
        )
      )
    end
    maybe_dump(now)
    return -1, timestamp, nil
  end

  st.count = st.count + 1
  st.bytes = st.bytes + nbytes
  st.total_count = (st.total_count or 0) + 1
  st.total_bytes = (st.total_bytes or 0) + nbytes
  dump_dirty = true
  maybe_dump(now)
  -- 1 = use this record (applies the leading-slash normalization).
  -- 0 would discard record mutations and keep Docker's "/name".
  return 1, timestamp, record
end

reload_config()
pcall(clear_stale_dump, DEFAULT_STATS_PATH)

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
  byte_cap_for = byte_cap_for,
  record_log_bytes = record_log_bytes,
  stats_for = stats_for,
  encode_stats = encode_stats,
  dump_stats = dump_stats,
  set_stats_path = function(p)
    stats_path = p or DEFAULT_STATS_PATH
  end,
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
    byte_cap = DEFAULT_BYTE_CAP,
    interval = DEFAULT_INTERVAL,
    max_keys = DEFAULT_MAX_KEYS,
  },
}
