---Health check for Hermes
---Run with :checkhealth hermes
local M = {}

---Format bytes to human-readable string
---@param bytes number
---@return string
local function format_bytes(bytes)
	if bytes < 1024 then
		return bytes .. " B"
	elseif bytes < 1024 * 1024 then
		return string.format("%.1f KB", bytes / 1024)
	else
		return string.format("%.1f MB", bytes / (1024 * 1024))
	end
end

---Walk a directory and return all file entries
---@param dir string
---@return table
local function scan_dir(dir)
	local entries = {}
	local handle = vim.uv.fs_scandir(dir)
	if not handle then
		return entries
	end
	while true do
		local name, typ = vim.uv.fs_scandir_next(handle)
		if not name then
			break
		end
		table.insert(entries, { name = name, type = typ })
	end
	return entries
end

---Check if a path is executable (cross-platform)
---@param path string
---@return boolean
local function is_executable(path)
	if vim.fn.has("win32") == 1 or vim.fn.has("win64") == 1 then
		return vim.fn.filereadable(path) == 1
	else
		return vim.fn.executable(path) == 1
	end
end

---Check if a directory is writable using libuv fs_access
---@param path string
---@return boolean
local function is_writable(path)
	local ok, can_write = pcall(vim.uv.fs_access, path, "W")
	return ok and can_write == true
end

---Check if a directory is writable or can be created (parent writable)
---@param dir string
---@return boolean
local function is_writable_or_creatable(dir)
	if vim.fn.isdirectory(dir) == 1 then
		return is_writable(dir)
	end
	-- Directory doesn't exist yet — check if parent is writable so we can create it
	local parent = vim.fn.fnamemodify(dir, ":h")
	return is_writable(parent)
end

---Pretty-print configuration as an array of group strings.
---Each group is a multi-line string where line 1 is the header and lines 2+ are indented.
---This works around Neovim's vim.health.info which strips leading whitespace from line 1.
---@param cfg table The full config table
---@return string[] Array of group strings
local function pretty_print_config(cfg)
	local groups = {}
	local current_group = {}

	local function add_line(indent, text)
		table.insert(current_group, string.rep("  ", indent) .. text)
	end

	local function flush_group()
		if #current_group > 0 then
			table.insert(groups, table.concat(current_group, "\n"))
			current_group = {}
		end
	end

	-- Permissions
	if cfg.permissions then
		add_line(0, "Permissions:")
		for k, v in pairs(cfg.permissions) do
			add_line(1, k .. ": " .. tostring(v))
		end
		flush_group()
	end

	-- Terminal
	if cfg.terminal then
		add_line(0, "Terminal:")
		for k, v in pairs(cfg.terminal) do
			add_line(1, k .. ": " .. tostring(v))
		end
		flush_group()
	end

	-- Buffer
	if cfg.buffer then
		add_line(0, "Buffer:")
		for k, v in pairs(cfg.buffer) do
			add_line(1, k .. ": " .. tostring(v))
		end
		flush_group()
	end

	-- Session
	if cfg.session then
		add_line(0, "Session:")
		for k, v in pairs(cfg.session) do
			add_line(1, k .. ": " .. tostring(v))
		end
		flush_group()
	end

	-- Distributions
	if cfg.distributions then
		add_line(0, "Distributions:")
		if cfg.distributions.binary then
			local b = cfg.distributions.binary
			add_line(1, "binary: enabled=" .. tostring(b.enabled) .. ", path=\"" .. (b.path or "") .. "\"")
		end
		add_line(1, "uvx: " .. tostring(cfg.distributions.uvx))
		add_line(1, "npx: " .. tostring(cfg.distributions.npx))
		flush_group()
	end

	-- Root Markers
	if cfg.root_markers then
		add_line(0, "Root Markers: " .. table.concat(cfg.root_markers, ", "))
		flush_group()
	end

	-- Log: File
	if cfg.log and cfg.log.file then
		add_line(0, "Log (File):")
		local f = cfg.log.file
		add_line(1, "level: " .. tostring(f.level))
		add_line(1, "format: " .. tostring(f.format))
		add_line(1, "path: " .. tostring(f.path))
		add_line(1, "name: " .. tostring(f.name))
		if f.max_size then
			add_line(1, "max_size: " .. format_bytes(f.max_size))
		end
		if f.max_files then
			add_line(1, "max_files: " .. tostring(f.max_files))
		end
		flush_group()
	end

	-- Log: Notification
	if cfg.log and cfg.log.notification then
		add_line(0, "Log (Notification):")
		add_line(1, "level: " .. tostring(cfg.log.notification.level))
		add_line(1, "format: " .. tostring(cfg.log.notification.format))
		flush_group()
	end

	-- Log: Stdio
	if cfg.log and cfg.log.stdio then
		add_line(0, "Log (Stdio):")
		add_line(1, "level: " .. tostring(cfg.log.stdio.level))
		add_line(1, "format: " .. tostring(cfg.log.stdio.format))
		flush_group()
	end

	-- Log: Message
	if cfg.log and cfg.log.message then
		add_line(0, "Log (Message):")
		add_line(1, "level: " .. tostring(cfg.log.message.level))
		add_line(1, "format: " .. tostring(cfg.log.message.format))
		flush_group()
	end

	return groups
end

M.check = function()
	-- =========================================================================
	-- Neovim Version
	-- =========================================================================
	vim.health.start("Neovim")
	if vim.fn.has("nvim-0.11") == 1 then
		vim.health.ok("Neovim >= 0.11")
	else
		vim.health.error("Neovim >= 0.11 is required")
	end

	-- =========================================================================
	-- Hermes Binary
	-- =========================================================================
	vim.health.start("Hermes Binary")
	local hermes = require("hermes")
	local state = hermes.get_loading_state()
	local error_msg = hermes.get_loading_error()
	local binary = require("hermes.binary")
	local bin_path = binary.get_binary_path()

	vim.health.info("Path: " .. bin_path)

	if state == "READY" then
		vim.health.ok("Binary loaded and ready")
	elseif state == "FAILED" then
		vim.health.error("Binary loading failed")
		if error_msg then
			vim.health.info("Error: " .. vim.inspect(error_msg))
		end
	elseif state == "DOWNLOADING" then
		vim.health.warn("Binary is being downloaded...")
	elseif state == "LOADING" then
		vim.health.warn("Binary is loading...")
	else
		vim.health.info("Binary not loaded yet — run any Hermes API method to start loading")
	end

	if vim.fn.filereadable(bin_path) == 1 then
		local size = vim.fn.getfsize(bin_path)
		vim.health.ok("Binary exists (" .. format_bytes(size) .. ")")
	else
		vim.health.error("Binary not found — will download on first use")
	end

	-- =========================================================================
	-- Version
	-- =========================================================================
	vim.health.start("Version")
	local version = require("hermes.version")
	local wanted = version.get_wanted()
	vim.health.info("Wanted: " .. wanted)

	local config = require("hermes.config")
	local download_cfg = config.get_download()
	local ver_file = binary.get_version_file()

	if download_cfg and download_cfg.auto == false then
		vim.health.info("Built from source (auto-download disabled)")
	elseif vim.fn.filereadable(ver_file) == 1 then
		local ok, installed = pcall(function()
			return vim.fn.readfile(ver_file)[1]
		end)
		if ok and installed and installed ~= "" then
			vim.health.info("Installed: " .. installed)
			if wanted == installed or wanted == "latest" then
				vim.health.ok("Versions match")
			else
				vim.health.warn("Wanted version differs from installed")
			end
		else
			vim.health.warn("Installed version unknown (version file empty)")
		end
	else
		vim.health.warn("Installed version unknown (no version file)")
	end

	-- =========================================================================
	-- Platform
	-- =========================================================================
	vim.health.start("Platform")
	local platform = require("hermes.platform")
	local os_name = platform.get_os() or "unknown"
	local arch = platform.get_arch() or "unknown"
	local key = platform.get_platform_key() or "unknown"

	vim.health.info("OS: " .. os_name)
	vim.health.info("Architecture: " .. arch)
	vim.health.info("Platform Key: " .. key)

	if platform.is_supported() then
		vim.health.ok("Platform is supported")
	else
		vim.health.error("Platform is not supported")
	end

	-- =========================================================================
	-- Download Tools
	-- =========================================================================
	vim.health.start("Download Tools")
	local download = require("hermes.download")
	if download.is_curl_available() then
		vim.health.ok("curl")
	else
		vim.health.warn("curl not found")
	end
	if download.is_wget_available() then
		vim.health.ok("wget")
	else
		vim.health.info("wget not found")
	end
	if download.is_powershell_available() then
		vim.health.ok("PowerShell")
	else
		vim.health.info("PowerShell not found")
	end

	-- =========================================================================
	-- Log Files
	-- =========================================================================
	vim.health.start("Log Files")
	local full_config = config.get()
	local log_dir = full_config.log.file.path
	local file_level = full_config.log.file.level
	local file_logging_enabled = file_level ~= "off" and file_level ~= 0

	vim.health.info("Directory: " .. log_dir)

	-- Check directory writability
	if not is_writable_or_creatable(log_dir) then
		if file_logging_enabled then
			vim.health.error("Log directory is not writable")
		else
			vim.health.warn("Log directory is not writable")
		end
	else
		vim.health.ok("Log directory is writable")
	end

	-- List existing log files
	local log_files = {}
	local total_size = 0
	if vim.fn.isdirectory(log_dir) == 1 then
		for _, entry in ipairs(scan_dir(log_dir)) do
			if entry.type == "file" and entry.name:match("^hermes%.log") then
				local path = log_dir .. "/" .. entry.name
				local stat = vim.uv.fs_stat(path)
				if stat then
					table.insert(log_files, entry.name .. " (" .. format_bytes(stat.size) .. ")")
					total_size = total_size + stat.size
				end
			end
		end
	end

	if #log_files > 0 then
		vim.health.ok(#log_files .. " log file(s) found, total " .. format_bytes(total_size))
		for _, info in ipairs(log_files) do
			vim.health.info("  " .. info)
		end
	else
		vim.health.info("No log files found")
	end

	if file_logging_enabled then
		vim.health.ok("File logging enabled (level: " .. tostring(file_level) .. ")")
	else
		vim.health.info("File logging is disabled (level: off)")
	end

	-- =========================================================================
	-- Registry Binaries
	-- =========================================================================
	vim.health.start("Registry Binaries")
	local cache_dir = config.get_registry_cache_dir()
	local binary_dist_enabled = full_config.distributions
		and full_config.distributions.binary
		and full_config.distributions.binary.enabled == true

	vim.health.info("Cache directory: " .. cache_dir)

	-- Check cache directory writability
	if not is_writable_or_creatable(cache_dir) then
		if binary_dist_enabled then
			vim.health.error("Registry cache directory is not writable")
		else
			vim.health.warn("Registry cache directory is not writable")
		end
	else
		vim.health.ok("Registry cache directory is writable")
	end

	local agents_found = {}
	if vim.fn.isdirectory(cache_dir) == 1 then
		for _, agent_entry in ipairs(scan_dir(cache_dir)) do
			if agent_entry.type == "directory" then
				local agent_path = cache_dir .. "/" .. agent_entry.name
				local versions = scan_dir(agent_path)
				for _, ver_entry in ipairs(versions) do
					if ver_entry.type == "directory" then
						local ver_path = agent_path .. "/" .. ver_entry.name
						local bin_entries = scan_dir(ver_path)
						for _, file_entry in ipairs(bin_entries) do
							if file_entry.type == "file" then
								local registry_bin_path = ver_path .. "/" .. file_entry.name
								local stat = vim.uv.fs_stat(registry_bin_path)
								local size_str = stat and format_bytes(stat.size) or "unknown size"
								local exec = is_executable(registry_bin_path)
								table.insert(
									agents_found,
									{
										name = agent_entry.name,
										version = ver_entry.name,
										binary = file_entry.name,
										size = size_str,
										executable = exec,
									}
								)
							end
						end
					end
				end
			end
		end
	end

	if #agents_found > 0 then
		vim.health.ok(#agents_found .. " registry binary(ies) found")
		for _, info in ipairs(agents_found) do
			local status = info.executable and "ok" or "warn"
			local msg = string.format(
				"  %s@%s: %s (%s)%s",
				info.name,
				info.version,
				info.binary,
				info.size,
				info.executable and "" or " — not executable"
			)
			if status == "ok" then
				vim.health.ok(msg)
			else
				vim.health.warn(msg)
			end
		end
	else
		vim.health.info("No registry binaries installed")
	end

	-- =========================================================================
	-- Configuration
	-- =========================================================================
	vim.health.start("Configuration")
	local groups = pretty_print_config(full_config)
	for _, group in ipairs(groups) do
		vim.health.info(group)
	end
end

return M
