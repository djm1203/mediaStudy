---
title: "RELEASE PROCESS"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-06-06T03:40:25Z
product_id: project
project_id: project
file_kind: ReleaseProcess
author: 
---

# Release Process — project

## Versioning

Follow Semantic Versioning (SemVer): MAJOR.MINOR.PATCH.

## Pre-Release Checklist

- [ ] All tests pass
- [ ] No known critical bugs
- [ ] CHANGELOG updated
- [ ] Version numbers bumped
- [ ] Documentation current

## Release Steps

1. Create release branch from main
2. Run full test suite
3. Update version numbers
4. Update CHANGELOG
5. Create tag
6. Build release artifacts
7. Publish

## Hotfix Process

1. Branch from release tag
2. Fix the issue
3. Create PATCH release
4. Cherry-pick fix to main
