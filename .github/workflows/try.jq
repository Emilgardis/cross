# Renders the `/ci try` conclusion comment.
#
# input: `gh run view --json jobs`
# $pr_id: pr number
# $comment: url to the try comment
# $try_error: JSON with `error` and `help` if the command was invalid, otherwise empty
# $annotations: object of job id -> annotations of the job

# markdown
def indent($s): split("\n") | map(if . == "" then . else $s + . end) | join("\n");
def item: "- " + (indent("  ") | .[2:]);
def code_block: select(. != null) | "```\n\(.)\n```";
def details($summary; $body): "<details>\n<summary>\($summary)</summary>\n\n\($body)\n\n</details>";
# table of links without a header
def grid:
  [range(0; length; 4) as $i | .[$i:$i + 4]] |
  ["| | | | |", "|-|-|-|-|"] + map("| " + join(" | ") + " |") | join("\n");

# jobs
def failed: .conclusion | IN("failure", "cancelled", "timed_out", "startup_failure");
def box: .conclusion | if . == "success" then "✔️" elif . == "skipped" then "🛇" else "❌" end;
def step_anchor(f): [.steps[] | select(f)] | first | if . then "#step:\(.number):1" else "" end;
def failed_step: [.steps[] | select(.conclusion == "failure")] | first;
def link:
  (if failed then ""
   else step_anchor((.name | startswith("Test")) and .conclusion == "success") end) as $anchor |
  "[\(.name)](\(.url)?pr=\($pr_id)\($anchor))";
def job_name: .name | ltrimstr("try / ") | (capture("^target \\((?<n>[^,]+),") | .n) // .;

# the error from xtask's `::error title=<target> <step>::` annotation, see `Steps::step` in
# xtask/src/test_util.rs, or `::error title=<target>::` from `cargo xtask build-docker-image`.
# the message is a summary, then the last lines of the output
def error_annotation:
  .name as $name |
  [($annotations[.id | tostring] // [])[] |
    select(.annotation_level == "failure") |
    (.title // "" | split(" ")) as $words |
    select(($words[0] // "") != "" and ($name | startswith($words[0]))) |
    {target: $words[0], step: ($words[1:] | join(" ")), output: (.message | split("\n")[1:])}
  ] | first;
def is_running: test("^\\s+Running ");
# cargo's status lines say nothing about the failure, except `Running` which gives some context
def is_cargo_status: test("^\\s+[A-Z][a-z]+ ") and (is_running | not);
def cause_lines:
  map(select(test("\\S") and (is_cargo_status | not))) |
  [.[] | select(startswith("error"))] as $errors |
  if $errors | length > 0 then $errors else .[-3:] end;
def text_or_null: join("\n") | if . == "" then null else . end;
def failed_entry:
  error_annotation as $error |
  failed_step as $step |
  ($error.output // [] | cause_lines) as $cause |
  ($cause | map(split($error.target) | join("<target>"))) as $generic |
  . + {
    step: (($step.name // .conclusion) + if ($error.step // "") != "" then " > \($error.step)" else "" end),
    step_number: $step.number,
    cause: ($cause | text_or_null),
    generic: ($generic | text_or_null),
    key: ($generic | map(select(is_running | not)) | text_or_null)
  };

# failed jobs of one step
# the cause of jobs grouped by key, as generic as it needs to be to be the same for all
def group_cause:
  if (map(.cause) | unique | length) == 1 then .[0].cause
  elif (map(.generic) | unique | length) == 1 then .[0].generic
  else .[0].key end;
def failed_group:
  .[0].step as $step |
  (.[0] | box) as $box |
  length as $n |
  (group_by(.key) | sort_by(-length)) as $by_cause |
  if ($by_cause | length) == 1 then
    group_cause as $cause |
    if $n > 5 then
      ["\($box) **\($step)** (\($n))", (map(link) | grid), ($cause | code_block)] | join("\n\n")
    else
      ["\($box) **\($step)** (\($n)): \(map(link) | join(", "))", ($cause | code_block)] | join("\n")
    end
  else
    ($by_cause | map([(map(link) | join(", ")), (group_cause | code_block)] | join("\n") | item) | join("\n")) as $list |
    if ($by_cause | length) > 3 then details("\($box) <b>\($step)</b> (\($n))"; $list)
    else "\($box) **\($step)** (\($n))\n\($list)"
    end
  end | item;
def failed_groups:
  group_by(.step) | sort_by((.[0].target | not), .[0].step_number // infinite, -length) |
  map(failed_group) | join("\n");

def invalid_command:
  fromjson | [(.error | code_block), details("Usage"; .help | rtrimstr("\n") | code_block)] | join("\n\n");

[.jobs[] | select(.name | startswith("try / ")) | select(.name | contains("matrix.pretty") | not) |
  {name: job_name, url, conclusion, steps, id: .databaseId, target: (.name | startswith("try / target"))}] as $jobs |
($jobs[] | select(.name == "conclusion")) as $conclusion |
($jobs[] | select(.name == "generate-matrix")) as $matrix |
[$jobs[] | select(.name != "conclusion") | select(failed) | failed_entry] as $failed |
[$jobs[] | select(.target and .conclusion == "success")] as $succeeded |

[
  "<!--try-conclusion-comment-->\n" +
  "## [Try](\($conclusion.url | sub("/job/.*"; ""))?pr=\($pr_id)) run for [comment](\($comment)) - \($conclusion | box)",
  "[Matrix](\($matrix.url)?pr=\($pr_id)\($matrix | step_anchor(.name == "Generate matrix")))",
  (select($try_error != "") | "#### Invalid Command\n\n" + ($try_error | invalid_command)),
  ($failed | select(length > 0) | "#### Failed Jobs (\(length))\n\n" + failed_groups),
  ($succeeded | select(length > 0) |
    "#### Successful Jobs (\(length))\n\n" +
    (map(link) | if length > 10 then details("List"; grid) else join(", ") end))
] | join("\n\n")
