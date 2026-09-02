+++
base = "fake-php"
intent = "Upgrade the dependencies of this project to newer versions."
allowed_tools = ["Bash", "Read", "Write", "Edit", "Grep"]

[[graders]]
type = "file_content"
path = "composer.json"
pattern = "1\\.0\\.0"
match = "not_contains"

[[graders]]
type = "file_content"
path = "composer.json"
pattern = "acme/example"

[[graders]]
type = "tool_used"
tool = "Edit"
+++

The agent reports that it raised `acme/example` from `1.0.0` to a newer
minor version, and it gives both versions.
