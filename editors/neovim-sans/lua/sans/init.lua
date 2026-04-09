local M = {}

function M.setup(opts)
  opts = opts or {}
  local cmd = opts.cmd or { "sans-lsp" }

  vim.api.nvim_create_autocmd("FileType", {
    pattern = "sans",
    callback = function(args)
      vim.lsp.start({
        name = "sans-lsp",
        cmd = cmd,
        root_dir = vim.fs.dirname(
          vim.fs.find({ "sans.json" }, { upward = true, path = vim.api.nvim_buf_get_name(args.buf) })[1]
        ) or vim.fn.getcwd(),
      })
    end,
  })
end

return M
