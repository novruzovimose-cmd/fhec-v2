import test from "node:test";
import assert from "node:assert/strict";
import { validatePublicCi, checkPublicCi } from "./github.mjs";

const sha = "0123456789abcdef0123456789abcdef01234567";
const repo = "example/fhec-v2";
const prefix = `https://github.com/${repo}/actions/runs/256`;
const metadata = { private: false, archived: false, full_name: repo };
const run = {
  id: 256,
  head_sha: sha,
  status: "completed",
  conclusion: "success",
  event: "push",
  path: `${repo}/.github/workflows/fhec-cofhe-phase13-external.yml@refs/heads/main`,
  html_url: prefix,
  repository: { full_name: repo },
  head_repository: { full_name: repo },
};
const jobs = [
  { id: 501, name: "Phase 13 offline and mock on ubuntu-latest", status: "completed", conclusion: "success", html_url: prefix + "/job/501" },
  { id: 502, name: "Phase 13 offline and mock on windows-latest", status: "completed", conclusion: "success", html_url: prefix + "/job/502" },
];

test("public CI verifier confirms two distinct hosted OS success jobs", () => {
  const result = validatePublicCi(repo, "256", sha, metadata, run, jobs);
  assert.equal(result.verified_jobs.length, 2);
  assert.equal(result.head_sha, sha);
});

test("CI refuses private repo, wrong commit, wrong workflow, untrusted run result", () => {
  assert.throws(() => validatePublicCi(repo, "256", sha, { ...metadata, private: true }, run, jobs));
  assert.throws(() => validatePublicCi(repo, "256", "A".repeat(40), metadata, run, jobs));
  assert.throws(() => validatePublicCi(repo, "256", sha, metadata, { ...run, conclusion: "failure" }, jobs));
  assert.throws(() => validatePublicCi(repo, "256", sha, metadata, { ...run, path: "wrong.yml" }, jobs));
  assert.throws(() => validatePublicCi(repo, "256", sha, metadata, { ...run, event: "pull_request" }, jobs));
});

test("CI refuses absent Windows job, failed job and reused job ID", () => {
  assert.throws(() => validatePublicCi(repo, "256", sha, metadata, run, jobs.slice(0, 1)));
  assert.throws(() => validatePublicCi(repo, "256", sha, metadata, run, [jobs[0], { ...jobs[1], conclusion: "skipped" }]));
  assert.throws(() => validatePublicCi(repo, "256", sha, metadata, run, [jobs[0], { ...jobs[1], id: jobs[0].id }]));
});

test("GitHub HTTP retrieval can verify the same public run", async () => {
  let queries = 0;
  const fetcher = async (url) => {
    queries += 1;
    const response = url.includes("/jobs?") ? { total_count: 2, jobs } :
      url.includes("/actions/runs/") ? run : metadata;
    return {
      ok: true,
      headers: { get: () => null },
      text: async () => JSON.stringify(response),
    };
  };
  const response = await checkPublicCi(repo, "256", sha, fetcher);
  assert.equal(response.checked_via_public_api, true);
  assert.equal(queries, 3);
});
