# Migration Guide

These guides upgrade an existing application to **fluent_typed_codegen 0.3.0**
and fluent-typed 0.10.1. You do not need to install intermediate generator
releases.

Choose the version you are upgrading from:

| Your current version | Start here |
| --- | --- |
| 0.2.2 | [Structured accessors, buffer ownership and validation](migration-0.2.2.md) |
| 0.2.0 or 0.2.1 | [Catalog paths, selective embedding and typed errors](migration-0.2.1.md) |
| 0.1.4 | [Explicit sources, checked loading and module catalogs](migration-0.1.4.md) |

Version 0.2.1 only changed the embedded lookup implementation, so applications
using 0.2.0 follow the same guide. All versions before 0.2.2 also need the
[fluent-typed 0.10 changes](migration-0.2.2.md); the earlier-version guides link
to the relevant steps.

After upgrading, regenerate the catalogs and run the application's translation
tests. Check structured messages, file or embedded loading, and rejection of
invalid external translations when those features are used.

[Quickstart](../../README.md#setup) · [Loading recipes](../loading.md) ·
[Changelog](../../CHANGELOG.md)
