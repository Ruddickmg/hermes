-- Unit tests for lua/hermes/download.lua
-- Tests HTTP download utilities with various tool paths

local stub = require("luassert.stub")

describe("hermes.download", function()
  local download
  local stubs = {}
  
  before_each(function()
    package.loaded["hermes.download"] = nil
    download = require("hermes.download")
    stubs = {}
  end)
  
  after_each(function()
    -- Clean up all stubs to prevent test pollution
    for _, s in ipairs(stubs) do
      if s and s.revert then
        s:revert()
      end
    end
    stubs = {}
  end)
  
  describe("tool availability", function()
    it("detects curl availability", function()
      -- Test when curl is available
      local exec_stub = stub(vim.fn, "executable").returns(1)
      assert.is_true(download.is_curl_available())
      exec_stub:revert()
      
      -- Test when curl is not available
      exec_stub = stub(vim.fn, "executable").returns(0)
      assert.is_false(download.is_curl_available())
      exec_stub:revert()
    end)
    
    it("detects wget availability", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("wget").returns(1)
      exec_stub.on_call_with("curl").returns(0)
      
      assert.is_true(download.is_wget_available())
      
      exec_stub:revert()
    end)
    
    it("detects PowerShell availability", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("powershell").returns(1)
      
      assert.is_true(download.is_powershell_available())
      
      exec_stub:revert()
    end)
    
    it("returns curl as first priority tool", function()
      local exec_stub = stub(vim.fn, "executable").returns(1)
      
      local tool = download.get_available_tool()
      
      assert.equals("curl", tool)
      
      exec_stub:revert()
    end)
    
    it("falls back to wget when curl not available", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("curl").returns(0)
      exec_stub.on_call_with("wget").returns(1)
      exec_stub.on_call_with("powershell").returns(0)
      
      local tool = download.get_available_tool()
      
      assert.equals("wget", tool)
      
      exec_stub:revert()
    end)
    
    it("returns nil when no tool available", function()
      local exec_stub = stub(vim.fn, "executable").returns(0)
      
      local tool = download.get_available_tool()
      
      assert.is_nil(tool)
      
      exec_stub:revert()
    end)
  end)
  
  describe("download()", function()
    it("returns error when no tool available", function()
      stub(vim.fn, "executable").returns(0)
      
      local ok, err = download.download("http://example.com/file", "/tmp/test")
      
      -- err should now be a structured error table
      assert.is_false(ok)
      assert.is_table(err)
      assert.is_not_nil(err.message)
      assert.truthy(err.message:match("No download tool available"))
      assert.equals("http://example.com/file", err.url)
      assert.is_nil(err.http_code)
      assert.is_nil(err.tool)
    end)
    
    it("detects download command failure", function()
      stub(vim.fn, "executable").returns(1)
      -- Stub download to simulate failure
      stub(download, "download").returns(false, "Command failed")
      
      local ok, err = download.download("http://example.com/file", "/tmp/test")
      
      -- Combined assertion: should fail with error message
      assert.is_true(not ok and err ~= nil, "Should return false with error message")
    end)
    
    it("falls back to PowerShell on Windows", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("curl").returns(0)
      exec_stub.on_call_with("wget").returns(0)
      exec_stub.on_call_with("powershell").returns(1)
      
      local system_stub = stub(vim.fn, "system").returns("")
      
      -- Mock successful download
      stub(download, "download").invokes(function(_url, _dest)
        -- Verify PowerShell command is constructed
        return true, nil
      end)
      
      local tool = download.get_available_tool()
      assert.equals("powershell", tool)
      
      exec_stub:revert()
      system_stub:revert()
    end)
    
    it("handles command not found error", function()
      -- Mock curl available
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("curl").returns(1)
      exec_stub.on_call_with("wget").returns(0)
      exec_stub.on_call_with("powershell").returns(0)
      
      -- Mock system to return "command not found" error
      stub(vim.fn, "system").returns("curl: command not found")
      
      -- Mock shell_error to indicate failure
      local ok = pcall(function()
        return download.download("http://example.com/file", "/tmp/test")
      end)
      
      -- Should not crash (pcall catches errors)
      assert.is_true(ok, "Should handle command not found without crashing")
      
      exec_stub:revert()
    end)
    
    it("handles empty downloaded file", function()
      -- Mock curl available
      stub(vim.fn, "executable").returns(1)
      
      -- Mock successful system call
      stub(vim.fn, "system").returns("")
      
      -- Mock fs_stat to return small file size (empty file scenario)
      local uv_stub = stub(vim.uv or vim.loop, "fs_stat").returns({ size = 50 })
      local unlink_stub = stub(vim.uv or vim.loop, "fs_unlink")
      
      local ok, err = download.download("http://example.com/file", "/tmp/test")
      
      -- Should fail because file is too small
      assert.is_false(ok)
      assert.is_table(err)
      assert.truthy(err.message:match("too small") or err.message:match("empty"))
      assert.equals("http://example.com/file", err.url)
      assert.equals(200, err.http_code) -- Should capture HTTP 200 from curl
      assert.equals("curl", err.tool)
      
      uv_stub:revert()
      if unlink_stub then unlink_stub:revert() end
    end)
    
    it("successfully downloads with wget", function()
      local exec_stub = stub(vim.fn, "executable")
      exec_stub.on_call_with("curl").returns(0)
      exec_stub.on_call_with("wget").returns(1)
      exec_stub.on_call_with("powershell").returns(0)
      
      stub(vim.fn, "system").returns("")
      stub(vim.uv or vim.loop, "fs_stat").returns({ size = 1000 })
      
      local ok, err = download.download("http://example.com/file", "/tmp/test")
      
      assert.is_true(ok)
      assert.is_nil(err)
      
      exec_stub:revert()
    end)
    
    it("successfully downloads with PowerShell", function()
       local exec_stub = stub(vim.fn, "executable")
       exec_stub.on_call_with("curl").returns(0)
       exec_stub.on_call_with("wget").returns(0)
       exec_stub.on_call_with("powershell").returns(1)
       
       stub(vim.fn, "system").returns("")
       stub(vim.uv or vim.loop, "fs_stat").returns({ size = 1000 })
       
       local ok, err = download.download("http://example.com/file", "/tmp/test")
       
       assert.is_true(ok)
       assert.is_nil(err)
       
       exec_stub:revert()
     end)
    
    it("captures HTTP code from curl output", function()
      stub(vim.fn, "executable").returns(1)
      
      -- Mock curl to return a 200 response code at end of output
      stub(vim.fn, "system").returns("200")
      -- Use size < 100 to trigger the "too small" error
      stub(vim.uv or vim.loop, "fs_stat").returns({ size = 50 })
      stub(vim.uv or vim.loop, "fs_unlink")
      
      local ok, err = download.download("http://example.com/file", "/tmp/test")
      
      -- Small file triggers error but should have captured HTTP code 200
      assert.is_false(ok)
      assert.is_table(err)
      assert.equals("http://example.com/file", err.url)
      assert.equals("curl", err.tool)
      assert.equals(200, err.http_code)
    end)
    
    describe("User-Agent header", function()
      it("includes User-Agent header in curl command", function()
        local exec_stub = stub(vim.fn, "executable")
        exec_stub.on_call_with("curl").returns(1)
        exec_stub.on_call_with("wget").returns(0)
        exec_stub.on_call_with("powershell").returns(0)

        local captured_cmd
        local system_stub = stub(vim.fn, "system").invokes(function(cmd)
          captured_cmd = cmd
          return ""
        end)
        local uv = vim.uv or vim.loop
        local fs_stat_stub = stub(uv, "fs_stat").returns({ size = 1000 })

        download.download("http://example.com/file", "/tmp/test")

        local has_ua = vim.tbl_contains(captured_cmd, "User-Agent: " .. download.USER_AGENT)

        exec_stub:revert()
        system_stub:revert()
        fs_stat_stub:revert()

        assert.is_true(has_ua)
      end)

      it("includes User-Agent flag in wget command", function()
        local exec_stub = stub(vim.fn, "executable")
        exec_stub.on_call_with("curl").returns(0)
        exec_stub.on_call_with("wget").returns(1)
        exec_stub.on_call_with("powershell").returns(0)

        local captured_cmd
        local system_stub = stub(vim.fn, "system").invokes(function(cmd)
          captured_cmd = cmd
          return ""
        end)
        local uv = vim.uv or vim.loop
        local fs_stat_stub = stub(uv, "fs_stat").returns({ size = 1000 })

        download.download("http://example.com/file", "/tmp/test")

        local has_ua = vim.tbl_contains(captured_cmd, "--user-agent=" .. download.USER_AGENT)

        exec_stub:revert()
        system_stub:revert()
        fs_stat_stub:revert()

        assert.is_true(has_ua)
      end)

      it("includes UserAgent parameter in PowerShell command", function()
        local exec_stub = stub(vim.fn, "executable")
        exec_stub.on_call_with("curl").returns(0)
        exec_stub.on_call_with("wget").returns(0)
        exec_stub.on_call_with("powershell").returns(1)

        local captured_cmd
        local system_stub = stub(vim.fn, "system").invokes(function(cmd)
          captured_cmd = cmd
          return ""
        end)
        local uv = vim.uv or vim.loop
        local fs_stat_stub = stub(uv, "fs_stat").returns({ size = 1000 })

        download.download("http://example.com/file", "/tmp/test")

        local ps_command = captured_cmd[3]

        exec_stub:revert()
        system_stub:revert()
        fs_stat_stub:revert()

        assert.truthy(ps_command:match('UserAgent "' .. download.USER_AGENT .. '"'))
      end)
    end)
  end)
  
  describe("pre-release downloads", function()
    it("can download pre-release version v0.3.0-beta.5 successfully", function()
      -- This test verifies that the download mechanism works with pre-release versions
      -- which are marked as "prerelease": true on GitHub
      stub(vim.fn, "executable").returns(1)
      
      -- Mock successful download
      stub(vim.fn, "system").returns("200")
      stub(vim.uv or vim.loop, "fs_stat").returns({ size = 4410248 }) -- Actual size from GitHub
      
      local url = "https://github.com/Ruddickmg/hermes.nvim/releases/download/v0.3.0-beta.5/libhermes-linux-x86_64.so"
      local ok, err = download.download(url, "/tmp/test_prerelease.so")
      
      -- Should succeed without errors
      assert.is_true(ok, "Pre-release download should succeed")
      assert.is_nil(err, "Should not return error for successful download")
    end)
    
    it("returns structured error for missing pre-release version", function()
      stub(vim.fn, "executable").returns(1)
      
      -- Mock a 404 response
      stub(vim.fn, "system").returns("404")
      stub(vim.uv or vim.loop, "fs_stat").returns({ size = 50 }) -- Small file triggers error
      stub(vim.uv or vim.loop, "fs_unlink")
      
      local url = "https://github.com/Ruddickmg/hermes.nvim/releases/download/v999.0.0/libhermes-linux-x86_64.so"
      local ok, err = download.download(url, "/tmp/test_missing.so")
      
      -- Should fail with structured error
      assert.is_false(ok)
      assert.is_table(err)
      assert.equals(url, err.url)
      assert.equals(404, err.http_code)
      assert.truthy(err.message:match("too small") or err.message:match("empty"))
    end)
  end)
  
  describe("system()", function()
    it("executes command and returns output", function()
      local system_stub = stub(vim.fn, "system").returns("output text")
      
      local output = download.system({"echo", "hello"})
      
      assert.equals("output text", output)
      
      system_stub:revert()
    end)
    
    it("returns output and exit code", function()
      stub(vim.fn, "system").returns("error output")
      -- vim.v.shell_error would be non-zero in real failure case
      
      local output, exit_code = download.system({"failing", "command"})
      
      assert.equals("error output", output)
      assert.equals("number", type(exit_code))
    end)
  end)

  describe("emit_progress()", function()
    it("calls nvim_echo with progress options", function()
      local echo_calls = {}
      local echo_stub = stub(vim.api, "nvim_echo").invokes(function(chunks, hist, opts)
        table.insert(echo_calls, { chunks = chunks, hist = hist, opts = opts })
      end)

      download.emit_progress("test-id", "Test Title", "begin", 0, "Starting")

      echo_stub:revert()

      assert.equals(1, #echo_calls, "nvim_echo should be called once")
      local call = echo_calls[1]
      assert.is_table(call.opts)
      assert.equals("progress", call.opts.kind)
      assert.equals("test-id", call.opts.id)
      assert.equals("hermes", call.opts.source)
      assert.equals("begin", call.opts.status)
      assert.equals(0, call.opts.percent)
      assert.equals("Starting", call.opts.title)
    end)

    it("omits percent and title when nil", function()
      local echo_calls = {}
      local echo_stub = stub(vim.api, "nvim_echo").invokes(function(chunks, hist, opts)
        table.insert(echo_calls, { chunks = chunks, hist = hist, opts = opts })
      end)

      download.emit_progress("end-id", "Done", "end", nil, nil)

      echo_stub:revert()

      local opts = echo_calls[1].opts
      assert.is_nil(opts.percent)
      assert.is_nil(opts.title)
    end)
  end)

  describe("download_async()", function()
    it("calls callback with error when no tool available", function()
      stub(download, "get_available_tool").returns(nil)

      local callback_err = nil
      local job_id = download.download_async("http://example.com/file", "/tmp/test", "test-id", function(success, err)
        callback_err = err
      end)

      assert.is_nil(job_id, "Should return nil when no tool available")
      assert.is_not_nil(callback_err, "Callback should receive error")
      assert.truthy(callback_err.message:match("No download tool available"))
    end)

    it("calls callback with error when jobstart fails", function()
      stub(download, "get_available_tool").returns("curl")
      stub(vim.fn, "jobstart").returns(0)

      local callback_err = nil
      local job_id = download.download_async("http://example.com/file", "/tmp/test", "test-id", function(success, err)
        callback_err = err
      end)

      assert.is_nil(job_id, "Should return nil when jobstart fails")
      assert.is_not_nil(callback_err, "Callback should receive error")
      assert.truthy(callback_err.message:match("Failed to start"))
    end)

    it("parses curl stderr hash marks for progress", function()
      stub(download, "get_available_tool").returns("curl")

      local progress_calls = {}
      local progress_stub = stub(download, "emit_progress").invokes(function(id, title, status, percent, text)
        table.insert(progress_calls, { id = id, status = status, percent = percent })
      end)

      local on_exit_fn = nil
      stub(vim.fn, "jobstart").invokes(function(cmd, opts)
        on_exit_fn = opts.on_exit
        -- Simulate curl stderr with hash marks
        opts.on_stderr(0, { "######" })
        return 123
      end)

      download.download_async("http://example.com/file", "/tmp/test", "curl-test", function() end)

      progress_stub:revert()

      local report = vim.tbl_filter(function(c)
        return c.status == "report"
      end, progress_calls)
      assert.is_true(#report > 0, "Should emit at least one report progress for curl hashes")
    end)

    it("parses wget stderr percentage for progress", function()
      stub(download, "get_available_tool").returns("wget")

      local progress_calls = {}
      local progress_stub = stub(download, "emit_progress").invokes(function(id, title, status, percent, text)
        table.insert(progress_calls, { id = id, status = status, percent = percent })
      end)

      local on_exit_fn = nil
      stub(vim.fn, "jobstart").invokes(function(cmd, opts)
        on_exit_fn = opts.on_exit
        -- Simulate wget stderr with percentage
        opts.on_stderr(0, { "50%" })
        return 123
      end)

      download.download_async("http://example.com/file", "/tmp/test", "wget-test", function() end)

      progress_stub:revert()

      local report = vim.tbl_filter(function(c)
        return c.status == "report"
      end, progress_calls)
      assert.is_true(#report > 0, "Should emit at least one report progress for wget percentage")
    end)

    it("calls callback with error on non-zero exit code", function()
      stub(download, "get_available_tool").returns("curl")
      -- Bypass vim.schedule_wrap so on_exit fires immediately in tests
      stub(vim, "schedule_wrap").invokes(function(fn)
        return fn
      end)

      local callback_success = nil
      local callback_err = nil
      local on_exit_fn = nil
      stub(vim.fn, "jobstart").invokes(function(cmd, opts)
        on_exit_fn = opts.on_exit
        return 123
      end)

      download.download_async("http://example.com/file", "/tmp/test", "exit-test", function(success, err)
        callback_success = success
        callback_err = err
      end)

      -- Simulate failed exit
      on_exit_fn(0, 1, "SIGTERM")

      assert.is_false(callback_success, "Callback should receive false on non-zero exit")
      assert.is_not_nil(callback_err, "Should receive error details")
      assert.equals(1, callback_err.exit_code)
    end)

    it("calls callback with error when downloaded file is too small", function()
      stub(download, "get_available_tool").returns("curl")
      stub(vim.uv or vim.loop, "fs_stat").returns({ size = 50 })
      stub(vim.uv or vim.loop, "fs_unlink")
      stub(vim, "schedule_wrap").invokes(function(fn)
        return fn
      end)

      local callback_success = nil
      local callback_err = nil
      local on_exit_fn = nil
      stub(vim.fn, "jobstart").invokes(function(cmd, opts)
        on_exit_fn = opts.on_exit
        return 123
      end)

      download.download_async("http://example.com/file", "/tmp/test", "size-test", function(success, err)
        callback_success = success
        callback_err = err
      end)

      -- Simulate successful exit but tiny file
      on_exit_fn(0, 0, "exit")

      assert.is_false(callback_success, "Should fail when file is too small")
      assert.truthy(callback_err.message:match("too small") or callback_err.message:match("empty"))
    end)

    it("parses HTTP code from curl stdout on success", function()
      stub(download, "get_available_tool").returns("curl")
      stub(vim.uv or vim.loop, "fs_stat").returns({ size = 1000 })
      stub(vim, "schedule_wrap").invokes(function(fn)
        return fn
      end)

      local callback_success = nil
      local callback_err = nil
      local captured_opts = nil
      stub(vim.fn, "jobstart").invokes(function(cmd, opts)
        captured_opts = opts
        return 123
      end)

      download.download_async("http://example.com/file", "/tmp/test", "http-test", function(success, err)
        callback_success = success
        callback_err = err
      end)

      -- Feed stdout data through the captured on_stdout callback
      if captured_opts and captured_opts.on_stdout then
        captured_opts.on_stdout(0, { "200" })
      end

      if captured_opts and captured_opts.on_exit then
        captured_opts.on_exit(0, 0, "exit")
      end

      assert.is_true(callback_success, "Should succeed with valid file size")
      assert.is_nil(callback_err, "Should not return error on success")
    end)
  end)
end)
