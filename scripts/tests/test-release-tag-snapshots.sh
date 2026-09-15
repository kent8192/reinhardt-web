#!/usr/bin/env bash
# Execute the release workflow's actual shell steps against disposable tag fixtures.
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
ruby - "$ROOT_DIR/.github/workflows/release-plz.yml" <<'RUBY'
require "yaml"
require "tmpdir"
require "open3"

steps = YAML.load_file(ARGV.fetch(0)).fetch("jobs").fetch("release-plz-release").fetch("steps")
snapshot = steps.find { |step| step["id"] == "tags-before" }.fetch("run")
check = steps.find { |step| step["id"] == "check-release" }.fetch("run")

# Only network access and injected failures are stubbed; tag enumeration,
# sorting, comparison, and workflow output handling use the real commands.
shim = <<'SH'
git() {
  if [ "$1" = fetch ]; then
    return "${FETCH_EXIT:-0}"
  fi
  if [ "$1" = tag ] && [ "${TAG_EXIT:-0}" != 0 ]; then
    return "$TAG_EXIT"
  fi
  command git "$@"
}
comm() {
  if [ "${COMM_EXIT:-0}" != 0 ]; then
    return "$COMM_EXIT"
  fi
  command comm "$@"
}
sort() {
  if [ "${SORT_EXIT:-0}" != 0 ]; then
    return "$SORT_EXIT"
  fi
  command sort "$@"
}
SH

cases = [
  ["new stable tag", %w[0.3.9 0.3.10 0.4.0-alpha.9 0.4.0-alpha.10], "0.3.17", nil, {}, "released=true\n"],
  ["new prerelease tag", %w[0.4.0-alpha.9], "0.4.0-alpha.10", nil, {}, "released=true\n"],
  ["unchanged tags", %w[0.3.9 0.3.10], nil, nil, {}, "released=false\n"],
  ["empty tags", [], nil, nil, {}, "released=false\n"],
  ["first tag", [], "0.3.17", nil, {}, "released=true\n"],
  ["missing snapshot", [], nil, :missing, {}, nil],
  ["comparison failure", [], "0.3.17", nil, {"COMM_EXIT" => "2"}, nil],
  ["fetch failure", [], nil, nil, {"FETCH_EXIT" => "1"}, nil],
  ["tag enumeration failure", [], nil, nil, {"TAG_EXIT" => "1"}, nil],
  ["sort failure", [], nil, nil, {"SORT_EXIT" => "1"}, nil],
]

cases.each do |name, versions, added, fault, overrides, expected|
  Dir.mktmpdir("release-tag-test-", "/tmp") do |dir|
    git = lambda do |*args|
      output, status = Open3.capture2e("git", *args, chdir: dir)
      raise output unless status.success?
    end
    git.call("init", "--quiet")
    git.call("-c", "user.name=Test", "-c", "user.email=test@example.invalid",
             "-c", "commit.gpgsign=false", "commit", "--quiet", "--allow-empty", "-m", "fixture")
    versions.each { |version| git.call("-c", "tag.gpgsign=false", "tag", "reinhardt-web@v#{version}") }
    output_file = File.join(dir, "output")
    before_file = File.join(dir, "tags-before-test.txt")
    env = {"RUNNER_TEMP" => dir, "GITHUB_RUN_ID" => "test", "GITHUB_OUTPUT" => output_file,
           "TAGS_BEFORE_FILE" => before_file, "LC_ALL" => "C"}
    log, status = Open3.capture2e(env, "bash", "-e", "-c", shim + snapshot, chdir: dir)
    raise "#{name}: snapshot failed: #{log}" unless status.success?
    raise "#{name}: wrong snapshot output" unless File.read(output_file) == "tags_file=#{before_file}\n"
    expected_tags = versions.map { |version| "reinhardt-web@v#{version}\n" }.sort.join
    raise "#{name}: snapshot is not in byte order" unless File.read(before_file) == expected_tags
    git.call("-c", "tag.gpgsign=false", "tag", "reinhardt-web@v#{added}") if added
    File.delete(before_file) if fault == :missing
    File.write(output_file, "")

    log, status = Open3.capture2e(env.merge(overrides), "bash", "-e", "-c", shim + check, chdir: dir)
    if expected
      raise "#{name}: check failed: #{log}" unless status.success?
      raise "#{name}: unexpected output: #{File.read(output_file).inspect}" unless File.read(output_file) == expected
    else
      raise "#{name}: failure was masked: #{log}" if status.success?
      raise "#{name}: error emitted a release decision" unless File.read(output_file) == ""
    end
    puts "PASS: #{name}"
  end
end
RUBY
