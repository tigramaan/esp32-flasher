# GitHub Pages verifier

Validates the static `site/` source before publication.

## Input

An optional path to the Pages directory. The default is `site`.

```powershell
node tools/site/verify.mjs
node tools/site/verify.mjs D:\work\esp32-flasher\site
```

The command is read-only; running it is equivalent to a dry run.

## Checks

- English-only public page;
- canonical, description, Open Graph, SoftwareApplication, and FAQ metadata;
- version-independent latest-release download URL;
- absence of unverified rating/download claims;
- responsive container, mobile breakpoint, tap target, and overflow contracts;
- robots, sitemap, `llms.txt`, and every referenced local asset.

## Output and errors

Success prints one `PASS` line and exits with code `0`. Failures are printed as
a deterministic list and exit with code `1`.
