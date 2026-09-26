# Changelog

All notable changes to **Quarry**. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this file is
materialized from the Pixygon Changelog API — edit there, not here.

## [0.5.1] — 2026-09-26

### Changed
- Documentation now clarifies the release process: this repo ships via `pearl ship` (tests, changelog, version bump, release, commit and push), while publishing to crates.io remains a separate manual step, and deploying is a separate step from shipping.

### Security
- Fixed a page-rendering issue where a specially crafted title or description in a published design could break out of the embedded data script and inject markup into the page. All embedded JSON is now escaped so it can no longer terminate its containing script tag.


