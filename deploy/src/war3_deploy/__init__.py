"""Dataset maintenance and publishing tools for the Warcraft III map archive.

Every command reads or rewrites the same content-addressed dataset, so the
catalog I/O, the atomic writes and the progress reporting live in
`war3_deploy.catalog` and `war3_deploy.progress` rather than being copied into
each command.
"""

__all__ = ["__version__"]

__version__ = "0.1.0"
