# Repolyzer
Git Repositry analyzer written in Rust for [Missing Semester](https://teaching.pages.sai.jku.at/missing-semester/) as exercise 1.

## Short Task Description:
Create statistics from, e.g., a big Git-Repository!

Use learned techniques or other possibilities, in order to visualize Git-Repositories in an interesting fashion.

[**Full Task Description**](https://teaching.pages.sai.jku.at/missing-semester/exercise/missing-semester-exercise1/) (*German*)

## Usage
```
Usage: repolyzer [OPTIONS] <PATH>

Analyze a Git repository and display statistics about it.

OPTIONS:
    -c, --commit-graph        Enable the commit graph (similar to GitHub's)
    -e, --extended-overview  *Enables the extended overview instead of the general one
    -n, --no-overview         Disable the general overview
    -p, --pie-chart           Enable the pie chart
    -w, --week-day-stats     *Enable the week day stats

Options marked with a '*' may take more time and resources to compute, depending on the size of the repository.

PATH:
    The path to the Git repository to analyze. This can be a local path or a remote URL.
    If a remote URL is provided, the repository will be cloned to a temporary directory.
```

## Note
This project is/was only tested on my local machine and may not work as intended on other systems.

---
*DISCLAIMER: This is a project for a university course. It is not intended to be used in production and may contain bugs or security vulnerabilities.*