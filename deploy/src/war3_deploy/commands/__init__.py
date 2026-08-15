"""One module per subcommand.

Each exposes `configure(parser)` to declare its arguments and `run(args)` to do
the work, which is all `war3_deploy.cli` needs to wire it up.
"""
