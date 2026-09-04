local function find_script()
  local candidates = {
    "rate_limit.lua",
    "fluent-bit/rate_limit.lua",
    "../rate_limit.lua",
  }
  for _, p in ipairs(candidates) do
    local f = io.open(p, "r")
    if f then
      f:close()
      return p
    end
  end
  error("could not find rate_limit.lua")
end

local SCRIPT = find_script()
dofile(SCRIPT)

local PASS = 1
local DROP = -1

local function rec(name, msg)
  return { container_name = name, log = msg or "hello" }
end

local function ts(sec)
  return { sec = sec, nsec = 0 }
end

local function collect_codes(name, n, t0, msg)
  local codes = {}
  for i = 1, n do
    local code = rate_limit("tag", ts(t0 or 100), rec(name, msg))
    codes[#codes + 1] = code
  end
  return codes
end

describe("rate_limit.lua source safety", function()
  it("does not call load, loadstring, or dofile", function()
    local f = assert(io.open(SCRIPT, "r"))
    local source = f:read("*a")
    f:close()
    assert.is_nil(source:match("[^%w_]load%s*%("), "must not call load()")
    assert.is_nil(source:match("loadstring%s*%("), "must not call loadstring()")
    assert.is_nil(source:match("dofile%s*%("), "must not call dofile()")
  end)
end)

describe("parse_overrides", function()
  it("parses name=integer pairs", function()
    local map = RateLimit.parse_overrides("jarvis=500,boltwall=50")
    assert.are.equal(500, map["jarvis"])
    assert.are.equal(50, map["boltwall"])
  end)

  it("trims whitespace around names and values", function()
    local map = RateLimit.parse_overrides(" jarvis = 500 , boltwall = 50 ")
    assert.are.equal(500, map["jarvis"])
    assert.are.equal(50, map["boltwall"])
  end)

  it("returns an empty table for nil or empty input", function()
    local empty = RateLimit.parse_overrides("")
    local n = 0
    for _ in pairs(empty) do
      n = n + 1
    end
    assert.are.equal(0, n)
    local none = RateLimit.parse_overrides(nil)
    n = 0
    for _ in pairs(none) do
      n = n + 1
    end
    assert.are.equal(0, n)
  end)

  it("rejects non name=integer and injection-style input", function()
    local rejected = {
      "foo=1;os.execute('rm -rf /')",
      "foo=load('evil')",
      "foo=dofile('/etc/passwd')",
      "=123",
      "foo=",
      "foo=12.5",
      "foo=-1",
      "foo=1e2",
      "foo=0x10",
      "bar=loadstring('print(1)')",
    }
    for _, s in ipairs(rejected) do
      local map = RateLimit.parse_overrides(s)
      assert.is_nil(map["foo"], "rejected input leaked foo: " .. s)
      assert.is_nil(map["bar"], "rejected input leaked bar: " .. s)
    end

    -- Valid entries are kept; invalid/injection entries in the same list are skipped.
    local mixed = RateLimit.parse_overrides(
      "container-1=50,bad,other=10,evil=load('x')"
    )
    assert.are.equal(50, mixed["container-1"])
    assert.are.equal(10, mixed["other"])
    assert.is_nil(mixed["bad"])
    assert.is_nil(mixed["evil"])
  end)
end)

describe("rate_limit filter", function()
  local notices

  before_each(function()
    notices = {}
    RateLimit.set_printer(function(msg)
      notices[#notices + 1] = msg
    end)
    RateLimit.set_env({
      FLUENTBIT_RATE_LIMIT_DEFAULT = "3",
      FLUENTBIT_RATE_LIMIT_INTERVAL = "10",
      FLUENTBIT_RATE_LIMIT_MAX_KEYS = "8",
      FLUENTBIT_RATE_LIMIT_OVERRIDES = "",
    })
  end)

  after_each(function()
    RateLimit.set_printer(nil)
    RateLimit.set_env(nil)
  end)

  it("passes records under the cap through", function()
    local record = rec("alpha", "hello")
    for _ = 1, 3 do
      local code, timestamp, out = rate_limit("tag", ts(100), record)
      assert.are.equal(PASS, code)
      assert.are.same(record, out)
      assert.are.equal("hello", out.log)
      assert.are.equal(100, timestamp.sec)
    end
  end)

  it("drops records over the cap", function()
    local codes = collect_codes("alpha", 5, 100)
    assert.are.same({ PASS, PASS, PASS, DROP, DROP }, codes)
  end)

  it("applies a per-container override instead of the global default", function()
    RateLimit.set_env({
      FLUENTBIT_RATE_LIMIT_DEFAULT = "2",
      FLUENTBIT_RATE_LIMIT_INTERVAL = "10",
      FLUENTBIT_RATE_LIMIT_MAX_KEYS = "8",
      FLUENTBIT_RATE_LIMIT_OVERRIDES = "noisy=5",
    })
    local quiet = collect_codes("quiet", 3, 100)
    local noisy = collect_codes("noisy", 6, 100)
    assert.are.same({ PASS, PASS, DROP }, quiet)
    assert.are.same({ PASS, PASS, PASS, PASS, PASS, DROP }, noisy)
  end)

  it("looks up overrides with or without a leading slash", function()
    RateLimit.set_env({
      FLUENTBIT_RATE_LIMIT_DEFAULT = "1",
      FLUENTBIT_RATE_LIMIT_INTERVAL = "10",
      FLUENTBIT_RATE_LIMIT_MAX_KEYS = "8",
      FLUENTBIT_RATE_LIMIT_OVERRIDES = "jarvis=4",
    })
    local codes = collect_codes("/jarvis", 5, 100)
    assert.are.same({ PASS, PASS, PASS, PASS, DROP }, codes)
  end)

  it("strips a leading slash from container_name for stream continuity", function()
    local record = rec("/jarvis", "hello")
    local code, _, out = rate_limit("tag", ts(100), record)
    assert.are.equal(PASS, code)
    assert.are.equal("jarvis", out.container_name)
    assert.is_true(RateLimit.has_key("jarvis"))
    assert.is_false(RateLimit.has_key("/jarvis"))
  end)

  it("isolates counters so one container's overage does not affect another", function()
    local a = collect_codes("a", 5, 100)
    local b = collect_codes("b", 2, 100)
    assert.are.same({ PASS, PASS, PASS, DROP, DROP }, a)
    assert.are.same({ PASS, PASS }, b)
  end)

  it("emits a first-breach-only notice per key per interval", function()
    collect_codes("alpha", 5, 100, "secret body")
    assert.are.equal(1, #notices)
    assert.is_not_nil(notices[1]:match("container alpha throttled"))
    assert.is_not_nil(notices[1]:match("count=3"))
    assert.is_not_nil(notices[1]:match("cap=3"))
    assert.is_nil(notices[1]:match("secret body"))

    -- still in the same 10s window starting at t=100
    collect_codes("alpha", 2, 105, "secret body")
    assert.are.equal(1, #notices)

    -- next aligned window (interval=10)
    collect_codes("alpha", 4, 110, "secret body")
    assert.are.equal(2, #notices)
    assert.is_nil(notices[2]:match("secret body"))
  end)

  it("does not re-inject the notice into the record stream", function()
    local code, _, out = rate_limit("tag", ts(100), rec("alpha", "hello"))
    assert.are.equal(PASS, code)
    assert.are.equal("hello", out.log)
    collect_codes("alpha", 4, 100)
    local drop_code, _, drop_out = rate_limit("tag", ts(100), rec("alpha", "hello"))
    assert.are.equal(DROP, drop_code)
    assert.is_nil(drop_out)
  end)

  it("bounds distinct keys by evicting the LRU entry", function()
    RateLimit.set_env({
      FLUENTBIT_RATE_LIMIT_DEFAULT = "1",
      FLUENTBIT_RATE_LIMIT_INTERVAL = "10",
      FLUENTBIT_RATE_LIMIT_MAX_KEYS = "3",
      FLUENTBIT_RATE_LIMIT_OVERRIDES = "",
    })
    rate_limit("t", ts(100), rec("k1"))
    rate_limit("t", ts(101), rec("k2"))
    rate_limit("t", ts(102), rec("k3"))
    assert.are.equal(3, RateLimit.key_count())
    rate_limit("t", ts(103), rec("k4"))
    assert.are.equal(3, RateLimit.key_count())
    assert.is_false(RateLimit.has_key("k1"))
    assert.is_true(RateLimit.has_key("k2"))
    assert.is_true(RateLimit.has_key("k3"))
    assert.is_true(RateLimit.has_key("k4"))
  end)

  it("falls back to the fluentd tag when container_name is absent", function()
    local code = rate_limit("fallback-tag", ts(100), { log = "hello" })
    assert.are.equal(PASS, code)
    assert.is_true(RateLimit.has_key("fallback-tag"))
  end)

  it("resets the counter on interval rollover so under-limit traffic passes again", function()
    local first = collect_codes("alpha", 4, 100)
    assert.are.same({ PASS, PASS, PASS, DROP }, first)
    local second = collect_codes("alpha", 3, 110)
    assert.are.same({ PASS, PASS, PASS }, second)
  end)
end)
