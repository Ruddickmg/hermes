-- luacov: disable
---HTTP download utilities
---@module hermes.download
---Provides a clean wrapper around HTTP download functionality with cross-platform support
-- luacov: enable
---Uses curl (Unix), wget (Unix fallback), or PowerShell (Windows)

local M = {}

local USER_AGENT = "hermes.nvim/0.1"
M.USER_AGENT = USER_AGENT

local PROGRESS_INTERVAL_MS = 50
M.PROGRESS_INTERVAL_MS = PROGRESS_INTERVAL_MS

-- luacov: disable
---Check if curl is available on the system
---@return boolean available
---@private
-- luacov: enable
function M.is_curl_available()
	return vim.fn.executable("curl") == 1
end

-- luacov: disable
---Check if wget is available on the system
---@return boolean available
---@private
-- luacov: enable
function M.is_wget_available()
	return vim.fn.executable("wget") == 1
end

-- luacov: disable
---Check if PowerShell is available (Windows)
---@return boolean available
---@private
-- luacov: enable
function M.is_powershell_available()
	return vim.fn.executable("powershell") == 1
end

-- luacov: disable
---Get available download tool
---Priority: curl (Unix) > wget (Unix) > PowerShell (Windows)
---@return string|nil tool_name Name of available tool, or nil if none
---@private
-- luacov: enable
function M.get_available_tool()
	if M.is_curl_available() then
		return "curl"
	elseif M.is_wget_available() then
		return "wget"
	elseif M.is_powershell_available() then
		return "powershell"
	end
	return nil
end

-- luacov: disable
---Emit a Neovim progress notification via nvim_echo(kind='progress') on 0.12+
---Also fires User Progress autocommand on all versions for a stable API
---@param id string Unique progress id
---@param title string Human-readable title
---@param status string "running", "success", or "failure"
---@param percent number|nil Percentage 0-100
---@param text string|nil Optional detail text
---@private
-- luacov: enable
function M.emit_progress(id, title, status, percent, text)
	local data = {
		id = id,
		title = title,
		source = "hermes",
		status = status,
	}
	if percent then
		data.percent = percent
	end
	if text then
		data.text = { text }
	end

	-- 0.12+: use nvim_echo for native progress integration (Progress event, UI events)
	if vim.fn.exists("+messagesopt") == 1 then
		local opts = {
			kind = "progress",
			id = id,
			source = "hermes",
			status = status,
		}
		if percent then
			opts.percent = percent
		end
		if text then
			opts.title = text
		end
		pcall(vim.api.nvim_echo, { { title, "" } }, false, opts)
	end

	-- All versions: fire User Progress autocommand for a stable, consistent API
	vim.api.nvim_exec_autocmds("User", { pattern = "Progress", data = data })
end

-- luacov: disable
---Get content length of a URL asynchronously via range request
---Uses a partial GET (bytes 0-1) to force CDN to include Content-Length/Content-Range
---@param url string URL to check
---@param callback function Callback function(content_length: number|nil)
---@return number|nil job_id The jobstart id, or nil if failed
---@private
-- luacov: enable
function M.get_content_length(url, callback)
	local tool = M.get_available_tool()

	if not tool then
		callback(nil)
		return nil
	end

	local cmd

	if tool == "curl" then
		-- -r 0-1: request first byte (forces CDN to return Content-Range/Content-Length)
		-- -s: silent, -L: follow redirects, -i: include headers
		cmd = { "curl", "-sL", "-i", "-r", "0-1", "-H", "User-Agent: " .. USER_AGENT, url }
	elseif tool == "wget" then
		-- --header="Range: bytes=0-1": send Range header
		-- --server-response: show headers, -O /dev/null: discard tiny body
		cmd = { "wget", "--header=Range: bytes=0-1", "--server-response", "--user-agent=" .. USER_AGENT, "-O", "/dev/null", url }
	else
		-- PowerShell: GET request with Range header
		local ps_cmd = string.format(
			'$headers = @{Range="bytes=0-1"}; Invoke-WebRequest -Uri "%s" -Headers $headers -UseBasicParsing -UserAgent "%s"',
			url,
			USER_AGENT
		)
		cmd = { "powershell", "-Command", ps_cmd }
	end

	local stdout_data = {}
	local stderr_data = {}

	local job_id = vim.fn.jobstart(cmd, {
		on_stdout = function(_, data)
			if data then
				for _, line in ipairs(data) do
					if line and line ~= "" then
						table.insert(stdout_data, line)
					end
				end
			end
		end,
		on_stderr = function(_, data)
			if data then
				for _, line in ipairs(data) do
					if line and line ~= "" then
						table.insert(stderr_data, line)
					end
				end
			end
		end,
		on_exit = vim.schedule_wrap(function(_, exit_code, _)
			if exit_code ~= 0 then
				callback(nil)
				return
			end

			local content_length = nil
			-- Combine stdout and stderr for parsing
			local all_output = table.concat(stdout_data, "\n") .. "\n" .. table.concat(stderr_data, "\n")

			-- Try Content-Range first (from 206 Partial Content response, e.g. "bytes 0-1/12345678")
			content_length = all_output:match("[Cc]ontent%-[Rr]ange:%s*bytes%s+%d+%-%d+%/(%d+)")
			if content_length then
				content_length = tonumber(content_length)
			end

			-- Fallback to Content-Length header
			if not content_length then
				content_length = all_output:match("[Cc]ontent%-[Ll]ength:%s*(%d+)")
				if content_length then
					content_length = tonumber(content_length)
				end
			end

			callback(content_length)
		end),
	})

	if job_id <= 0 then
		callback(nil)
		return nil
	end

	return job_id
end

-- luacov: disable
---Download file from URL using available tool (synchronous)
---Cross-platform: curl (Unix), wget (Unix fallback), PowerShell (Windows)
---@param url string URL to download
---@param dest_path string Destination path
---@return boolean success Whether download succeeded
---@return table|nil error Error info table if failed, containing:
---   - message: Human readable error message
---   - url: URL that was attempted
---   - http_code: HTTP status code (if available)
---   - tool: Which download tool was used
---   - exit_code: Shell exit code
---   - stderr: Raw error output from tool
---@private
-- luacov: enable
function M.download(url, dest_path)
	local tool = M.get_available_tool()

	if not tool then
		return false,
			{
				message = "No download tool available (tried curl, wget, PowerShell). Please install curl or wget.",
				url = url,
				http_code = nil,
				tool = nil,
				exit_code = nil,
				stderr = nil,
			}
	end

	local cmd
	local http_code = nil

	if tool == "curl" then
		cmd = { "curl", "-sL", "-H", "User-Agent: " .. USER_AGENT, "-o", dest_path, "-w", "%{http_code}", url }
	elseif tool == "wget" then
		cmd = { "wget", "-q", "--user-agent=" .. USER_AGENT, "-O", dest_path, url }
	else
		-- PowerShell for Windows
		local ps_cmd = string.format(
			'Invoke-WebRequest -Uri "%s" -OutFile "%s" -UseBasicParsing -UserAgent "%s"',
			url,
			dest_path,
			USER_AGENT
		)
		cmd = { "powershell", "-Command", ps_cmd }
	end

	local result = vim.fn.system(cmd)
	local exit_code = vim.v.shell_error

	-- For curl, extract HTTP code from the end of output (since we used -w %{http_code})
	if tool == "curl" and result then
		-- The HTTP code is appended to stdout after the file is written
		http_code = result:match("(%d%d%d)$")
		if http_code then
			http_code = tonumber(http_code)
		end
	end

	if exit_code ~= 0 then
		-- Check if it's a command not found error vs network error
		local error_msg = result
		if result:match("command not found") or result:match("not installed") or result:match("is not recognized") then
			error_msg = tool .. " appears to be installed but execution failed: " .. result
		end

		return false,
			{
				message = error_msg,
				url = url,
				http_code = http_code,
				tool = tool,
				exit_code = exit_code,
				stderr = result,
			}
	end

	-- Verify file was downloaded and has reasonable size using vim.uv for cross-platform compatibility
	local uv = vim.uv or vim.loop
	local stat = uv.fs_stat(dest_path)
	if not stat or stat.size < 100 then
		-- Use vim.uv.fs_unlink for cross-platform file deletion
		uv.fs_unlink(dest_path)
		return false,
			{
				message = "Downloaded file is too small or empty",
				url = url,
				http_code = http_code or 200,
				tool = tool,
				exit_code = 0,
				stderr = "File size: " .. (stat and stat.size or 0) .. " bytes",
			}
	end

	return true, nil
end

-- luacov: disable
---Download file asynchronously with progress reporting
---Uses jobstart for non-blocking download and emits nvim_echo(kind='progress')
---@param url string URL to download
---@param dest_path string Destination path
---@param progress_id string Unique id for progress tracking
---@param on_complete function Callback function(success: boolean, err: table|nil)
---@return number|nil job_id The jobstart id, or nil if failed to start
---@private
-- luacov: enable
function M.download_async(url, dest_path, progress_id, on_complete, title)
	title = title or "Downloading Hermes binary"
	local tool = M.get_available_tool()

	if not tool then
		on_complete(false, {
			message = "No download tool available (tried curl, wget, PowerShell). Please install curl or wget.",
			url = url,
		})
		return nil
	end

	local cmd
	local http_code_pattern = nil

	if tool == "curl" then
		-- -sL for silent, follow redirects; -w writes HTTP code to stdout
		cmd = {
			"curl",
			"-sL",
			"-H",
			"User-Agent: " .. USER_AGENT,
			"-o",
			dest_path,
			"-w",
			"%{http_code}",
			url,
		}
		http_code_pattern = "(%d%d%d)$"
	elseif tool == "wget" then
		-- wget with -q for quiet (no progress output)
		cmd = {
			"wget",
			"-q",
			"--user-agent=" .. USER_AGENT,
			"-O",
			dest_path,
			url,
		}
	else
		-- PowerShell for Windows
		local ps_cmd = string.format(
			'Invoke-WebRequest -Uri "%s" -OutFile "%s" -UseBasicParsing -UserAgent "%s"',
			url,
			dest_path,
			USER_AGENT
		)
		cmd = { "powershell", "-Command", ps_cmd }
	end

	local stdout_data = {}
	local stderr_data = {}
	local download_finished = false
	local total_size = nil

	M.emit_progress(progress_id, title, "running", 0, "Starting download...")

	local uv = vim.uv or vim.loop

	-- Get content length asynchronously
	M.get_content_length(url, function(size)
		total_size = size
	end)

	-- Start progress timer
	local timer = uv.new_timer()
	timer:start(PROGRESS_INTERVAL_MS, PROGRESS_INTERVAL_MS, function()
		if download_finished then
			timer:stop()
			timer:close()
			return
		end

		local stat = uv.fs_stat(dest_path)
		if stat and stat.size > 0 then
			local percent = nil
			if total_size and total_size > 0 then
				percent = math.floor((stat.size / total_size) * 100)
				percent = math.min(percent, 99) -- Don't show 100% until actually done
			end

			vim.schedule(function()
				M.emit_progress(progress_id, title, "running", percent, "Downloading...")
			end)
		end
	end)

	local job_id = vim.fn.jobstart(cmd, {
		on_stdout = function(_, data)
			if data then
				for _, line in ipairs(data) do
					if line and line ~= "" then
						table.insert(stdout_data, line)
					end
				end
			end
		end,
		on_stderr = function(_, data)
			if data then
				for _, line in ipairs(data) do
					if line and line ~= "" then
						table.insert(stderr_data, line)
					end
				end
			end
		end,
		on_exit = vim.schedule_wrap(function(_, exit_code, _)
			download_finished = true
			if timer then
				timer:stop()
				timer:close()
				timer = nil
			end

			if exit_code ~= 0 then
				local stderr_output = table.concat(stderr_data, "\n")
				M.emit_progress(
					progress_id,
					title,
					"failure",
					nil,
					"Download failed with exit code " .. exit_code
				)
				on_complete(false, {
					message = "Download failed with exit code: " .. exit_code,
					url = url,
					exit_code = exit_code,
					stderr = stderr_output,
				})
				return
			end

			-- Parse HTTP code for curl
			local http_code = nil
			if tool == "curl" and #stdout_data > 0 then
				local last_line = stdout_data[#stdout_data]
				http_code = last_line:match(http_code_pattern)
				if http_code then
					http_code = tonumber(http_code)
				end
			end

			-- Verify downloaded file
			local stat = uv.fs_stat(dest_path)
			if not stat or stat.size < 100 then
				uv.fs_unlink(dest_path)
				M.emit_progress(
					progress_id,
					title,
					"failure",
					nil,
					"Downloaded file is too small or empty"
				)
				on_complete(false, {
					message = "Downloaded file is too small or empty",
					url = url,
					http_code = http_code,
				})
				return
			end

			M.emit_progress(
				progress_id,
				title,
				"success",
				100,
				"Download finished successfully"
			)
			on_complete(true, nil)
		end),
	})

	if job_id <= 0 then
		download_finished = true
		if timer then
			timer:stop()
			timer:close()
			timer = nil
		end
		M.emit_progress(
			progress_id,
			title,
			"failure",
			nil,
			"Failed to start download job"
		)
		on_complete(false, {
			message = "Failed to start download job",
			url = url,
		})
		return nil
	end

	return job_id
end

-- luacov: disable
---Execute a shell command and return result
---Simple wrapper around vim.fn.system for consistency
---@param cmd table|string Command as array or string
---@return string output Command output
---@return number exit_code Exit code (0 = success)
---@private
-- luacov: enable
function M.system(cmd)
	local output = vim.fn.system(cmd)
	return output, vim.v.shell_error
end

return M
