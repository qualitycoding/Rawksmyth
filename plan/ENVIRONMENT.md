# Environment record

Execution environment classified **ephemeral** (sandbox recycles state; durable
mount used for all working state). Git transport to github.com is unreachable
from the sandbox; reads use codeload tarballs, writes use the GitHub Git
database API with verified remote SHAs after each push. Long builds, tests and
mutation runs are delegated to CI jobs (see plan/CI-MATRIX.md).
