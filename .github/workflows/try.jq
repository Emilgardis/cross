# Renders the `/ci try` conclusion comment.
#
# input: `gh run view --json jobs`
# $pr_id: pr number
# $comment: url to the try comment
def failed: .conclusion | IN("failure", "cancelled", "timed_out", "startup_failure");
def box: .conclusion | if . == "success" then "✔️" elif . == "skipped" then "🛇" else "❌" end;
def step_anchor(f): [.steps[] | select(f)] | first | if . then "#step:\(.number):1" else "" end;
def failed_step: [.steps[] | select(.conclusion == "failure")] | first;
def link:
  (if failed then ""
   else step_anchor((.name | startswith("Test")) and .conclusion == "success") end) as $anchor |
  "[\(.name)](\(.url)?pr=\($pr_id)\($anchor))";
# table of links without a header, `indent` is prepended to each line
def grid(indent):
  [range(0; length; 4) as $i | .[$i:$i + 4]] |
  ["| | | | |", "|-|-|-|-|"] + map("| " + join(" | ") + " |") | map(indent + .) | join("\n");
def job_name:.name | ltrimstr("try / ") | (capture("^target \\((?<n>[^,]+),") | .n) // .;

[.jobs[] | select(.name | startswith("try / ")) | select(.name | contains("matrix.pretty") | not) |
  {name: job_name, url, conclusion, steps, target: (.name | startswith("try / target"))}] as $jobs |
($jobs[] | select(.name == "conclusion")) as $conclusion |
($jobs[] | select(.name == "generate-matrix")) as $matrix |
[$jobs[] | select(.name != "conclusion") | select(failed) | . + {step: (failed_step.name // .conclusion)}] as $failed |
[$jobs[] | select(.target and .conclusion == "success")] as $succeeded |

"<!--try-conclusion-comment-->\n" +
"## [Try](\($conclusion.url | sub("/job/.*"; ""))?pr=\($pr_id)) run for [comment](\($comment)) - \($conclusion | box)\n\n" +
"[Matrix](\($matrix.url)?pr=\($pr_id)\($matrix | step_anchor(.name == "Generate matrix")))\n\n" +
(if $failed | length > 0 then
  "#### Failed Jobs (\($failed | length))\n\n" +
  ($failed | group_by(.step) | sort_by(-length) | map(
    if length > 5 then
      "- <details><summary><b>\(.[0].step)</b> (\(length))</summary>\n\n\(map(link) | grid("  "))\n\n  </details>"
    else
      "- **\(.[0].step)** (\(length)): \(map(link) | join(", "))"
    end
  ) | join("\n")) + "\n\n"
else "" end) +
(if $succeeded | length > 0 then
  "#### Successful Jobs (\($succeeded | length))\n\n" +
  if $succeeded | length > 10 then
    "<details>\n<summary>List</summary>\n\n\($succeeded | map(link) | grid(""))\n\n</details>\n"
  else
    ($succeeded | map(link) | join(", ")) + "\n"
  end
else "" end)
