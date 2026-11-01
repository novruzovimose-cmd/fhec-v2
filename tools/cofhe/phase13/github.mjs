const shaPattern = /^[0-9a-f]{40}$/i;
const repositoryPattern = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;
const expectedWorkflow = ".github/workflows/fhec-cofhe-phase13-external.yml";
const expectedLabels = ["ubuntu-latest", "windows-latest"];

function validUrl(value, root) {
  if (typeof value !== "string" || !value.startsWith(root)) throw new Error("Invalid public GitHub API response URL");
  return value;
}

export function validatePublicCi(repository, runId, commitSha, repositoryMetadata, workflowRun, jobs) {
  if (!repositoryPattern.test(repository) || !/^[1-9][0-9]{0,19}$/.test(String(runId)) ||
      !shaPattern.test(commitSha)) throw new Error("A public owner/repo, run ID and complete Git commit SHA are required");
  const [owner, repo] = repository.split("/");
  const reference = `https://github.com/${owner}/${repo}`;
  if (!repositoryMetadata || repositoryMetadata.private !== false || repositoryMetadata.full_name?.toLowerCase() !== repository.toLowerCase() ||
      repositoryMetadata.archived === true) throw new Error("Repository is not a public, non-archived matching repository");
  if (!workflowRun || workflowRun.id !== Number(runId) || workflowRun.head_sha?.toLowerCase() !== commitSha.toLowerCase() ||
      workflowRun.status !== "completed" || workflowRun.conclusion !== "success" ||
      !["push", "workflow_dispatch"].includes(workflowRun.event) ||
      !workflowRun.path?.split("@")[0]?.endsWith(expectedWorkflow) ||
      workflowRun.repository?.full_name?.toLowerCase() !== repository.toLowerCase() ||
      workflowRun.head_repository?.full_name?.toLowerCase() !== repository.toLowerCase()) {
    throw new Error("Hosted CI run does not prove the requested commit and workflow passed");
  }
  validUrl(workflowRun.html_url, `${reference}/actions/runs/`);
  if (!Array.isArray(jobs) || jobs.length < 2 || jobs.length > 1000) throw new Error("Incomplete hosted job list");
  const matched = [];
  for (const os of expectedLabels) {
    const specific = jobs.filter((job) => job.name === `Phase 13 offline and mock on ${os}`);
    if (specific.length !== 1 || specific[0].status !== "completed" || specific[0].conclusion !== "success" ||
        !Number.isSafeInteger(specific[0].id) || specific[0].id <= 0) {
      throw new Error(`Missing or unsuccessful independent ${os} hosted job`);
    }
    validUrl(specific[0].html_url, `${reference}/actions/runs/`);
    matched.push({ os, job_id: specific[0].id, conclusion: "success", url: specific[0].html_url });
  }
  if (matched[0].job_id === matched[1].job_id) throw new Error("Two OS jobs cannot reuse one job identifier");
  return {
    schema: "fhec-cofhe-phase13-public-github-ci-v1",
    evidence_tier: "github-api-observed-hosted-success",
    repository,
    head_sha: commitSha.toLowerCase(),
    run_id: Number(runId),
    run_url: workflowRun.html_url,
    verified_jobs: matched,
    checked_via_public_api: true,
    attestations_signed_by_user: false,
    limitations: [
      "GitHub API metadata establishes reported hosted job results, not a cryptographic proof of every workflow action.",
      "This does not prove deployed testnet execution or third-party security review.",
      "The reviewed source commit must match the distributed release source before grant publication.",
    ],
  };
}

export async function checkPublicCi(repository, runId, commitSha, request = fetch) {
  if (!repositoryPattern.test(repository) || !/^[1-9][0-9]{0,19}$/.test(String(runId)) || !shaPattern.test(commitSha)) {
    throw new Error("Invalid GitHub repo/run/SHA request");
  }
  const endpoint = `https://api.github.com/repos/${repository}`;
  const options = {
    headers: { accept: "application/vnd.github+json", "user-agent": "fhec-phase13-public-ci-verifier", "x-github-api-version": "2022-11-28" },
    signal: AbortSignal.timeout(15000),
  };
  async function getJson(path) {
    const response = await request(endpoint + path, options);
    if (!response.ok) throw new Error("GitHub public API request failed (HTTP " + response.status + ")");
    const length = Number(response.headers.get("content-length") || "0");
    if (length > 4 * 1024 * 1024) throw new Error("GitHub response exceeded size limit");
    const raw = await response.text();
    if (raw.length > 4 * 1024 * 1024) throw new Error("GitHub response exceeded size limit");
    return JSON.parse(raw);
  }
  const repo = await getJson("");
  const run = await getJson(`/actions/runs/${runId}`);
  const jobs = [];
  for (let page = 1; page <= 10; page += 1) {
    const reply = await getJson(`/actions/runs/${runId}/jobs?per_page=100&page=${page}`);
    if (!reply || !Array.isArray(reply.jobs) || reply.total_count > 1000) throw new Error("Invalid or oversized GitHub Actions jobs response");
    jobs.push(...reply.jobs);
    if (jobs.length >= reply.total_count) break;
    if (page === 10) throw new Error("Hosted CI job response was truncated");
  }
  return validatePublicCi(repository, runId, commitSha, repo, run, jobs);
}
