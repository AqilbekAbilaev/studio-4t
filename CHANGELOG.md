# Changelog

## v0.1.1

- **Reduced memory usage after large queries** — switched to the mimalloc
  allocator and stream query results as pre-serialized JSON, cutting retained
  memory after a big fetch by roughly two-thirds.
