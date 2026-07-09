// Client-side GitHub: connect via OAuth popup, then create a repo and push the
// whole project directly from the browser using the user's token. The token lives
// in memory for the duration of the push and is never stored or sent to us.

const GH_API = "https://api.github.com";

/** Open the GitHub OAuth popup and resolve with the user token. */
export function connectGitHub(clientId: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const state = crypto.randomUUID();
    const redirect = `${location.origin}/api/github/callback`;
    const url =
      `https://github.com/login/oauth/authorize?client_id=${clientId}` +
      `&scope=public_repo&redirect_uri=${encodeURIComponent(redirect)}&state=${state}`;

    const popup = window.open(url, "redstart-github", "width=720,height=820");
    if (!popup) {
      reject(new Error("Popup blocked — allow popups for this site and retry."));
      return;
    }

    const onMessage = (e: MessageEvent) => {
      if (e.origin !== location.origin) return;
      const d = e.data;
      if (!d || d.type !== "redstart-github") return;
      cleanup();
      if (d.error) reject(new Error(d.error));
      else if (d.state && d.state !== state) reject(new Error("State mismatch — please retry."));
      else if (d.token) resolve(d.token as string);
      else reject(new Error("No token returned."));
    };

    const timer = setInterval(() => {
      if (popup.closed) {
        cleanup();
        reject(new Error("Window closed before authorizing."));
      }
    }, 500);

    function cleanup() {
      clearInterval(timer);
      window.removeEventListener("message", onMessage);
    }
    window.addEventListener("message", onMessage);
  });
}

async function gh(
  token: string,
  path: string,
  body?: unknown,
  method?: string,
  label?: string,
) {
  const res = await fetch(`${GH_API}${path}`, {
    method: method ?? (body ? "POST" : "GET"),
    headers: {
      authorization: `Bearer ${token}`,
      accept: "application/vnd.github+json",
      "x-github-api-version": "2022-11-28",
      ...(body ? { "content-type": "application/json" } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    // GitHub's top-level `message` is often generic ("Repository creation failed.");
    // the real reason is in `errors[]`. Surface both so failures are actionable.
    const detail = Array.isArray(data?.errors)
      ? data.errors
          .map((e: { message?: string; field?: string; code?: string }) =>
            e.message ?? [e.field, e.code].filter(Boolean).join(" "),
          )
          .filter(Boolean)
          .join("; ")
      : "";
    // A 404/403 here is almost always a missing/cached scope. Surface which step
    // failed and what scopes the token actually carries, so it's actionable.
    const scopes = res.headers.get("x-oauth-scopes");
    const hint =
      (res.status === 404 || res.status === 403) && !hasRepoScope(scopes)
        ? ` — your GitHub token is missing repo access (scopes: ${scopes || "none"}). Revoke Redstart at github.com/settings/applications, then reconnect and approve the repository permission.`
        : "";
    const where = label ? `${label}: ` : "";
    const msg =
      where +
      ([data?.message, detail].filter(Boolean).join(" — ") || `GitHub API ${res.status}`) +
      hint;
    throw new Error(msg);
  }
  return data;
}

/** Whether an X-OAuth-Scopes header grants repo creation (`repo` or `public_repo`). */
function hasRepoScope(scopes: string | null): boolean {
  if (!scopes) return false;
  const set = scopes.split(",").map((s) => s.trim());
  return set.includes("repo") || set.includes("public_repo");
}

export interface CreatedRepo {
  url: string;
  fullName: string;
}

/** Create a public repo and push the whole project as a single initial commit. */
export async function createSubgraphRepo(
  token: string,
  repoName: string,
  files: Record<string, string>,
  description: string,
): Promise<CreatedRepo> {
  // 0. Preflight: confirm the token is live and actually carries repo scope. A
  // cached OAuth grant can hand back a token missing `public_repo`, which makes
  // `POST /user/repos` return a bare "404 Not Found" — check up front so the
  // error is "reconnect and approve repo access", not a mystery.
  const meRes = await fetch(`${GH_API}/user`, {
    headers: {
      authorization: `Bearer ${token}`,
      accept: "application/vnd.github+json",
      "x-github-api-version": "2022-11-28",
    },
  });
  if (!meRes.ok) {
    throw new Error(
      `auth check: GitHub ${meRes.status} — the connection didn't stick. Disconnect and reconnect GitHub.`,
    );
  }
  if (!hasRepoScope(meRes.headers.get("x-oauth-scopes"))) {
    throw new Error(
      `Your GitHub token can't create repositories (scopes: ${meRes.headers.get("x-oauth-scopes") || "none"}). Revoke Redstart at github.com/settings/applications, then reconnect and approve the repository permission.`,
    );
  }

  // 1. Create the repo with an initial commit. `auto_init: true` is required —
  // the Git Data API can't create blobs/trees on a bare repo ("Git Repository is
  // empty"), so GitHub must lay down a first commit to initialise the git backend.
  const repo = await gh(
    token,
    "/user/repos",
    { name: repoName, description, private: false, auto_init: true },
    "POST",
    "create repo",
  );
  const owner: string = repo.owner.login;
  const name: string = repo.name;
  const branch: string = repo.default_branch || "main";

  // 2. The base commit + its tree (to layer our files on top). The ref can lag a
  // moment after auto_init, so retry briefly.
  let ref;
  for (let i = 0; ; i++) {
    try {
      ref = await gh(token, `/repos/${owner}/${name}/git/ref/heads/${branch}`, undefined, undefined, "read branch");
      break;
    } catch (e) {
      if (i >= 4) throw e;
      await new Promise((r) => setTimeout(r, 800));
    }
  }
  const baseSha: string = ref.object.sha;
  const baseCommit = await gh(token, `/repos/${owner}/${name}/git/commits/${baseSha}`);

  // 3. Blob per file.
  const tree = [];
  for (const [path, content] of Object.entries(files)) {
    const blob = await gh(token, `/repos/${owner}/${name}/git/blobs`, {
      content,
      encoding: "utf-8",
    });
    tree.push({ path, mode: "100644", type: "blob", sha: blob.sha });
  }

  // 4. Tree (on the base tree) → 5. commit (on the initial commit) → 6. move the branch.
  const treeObj = await gh(token, `/repos/${owner}/${name}/git/trees`, {
    base_tree: baseCommit.tree.sha,
    tree,
  });
  const commit = await gh(token, `/repos/${owner}/${name}/git/commits`, {
    message: "Add subgraph — generated by The Generator (redstart-lang.com)",
    tree: treeObj.sha,
    parents: [baseSha],
  });
  await gh(
    token,
    `/repos/${owner}/${name}/git/refs/heads/${branch}`,
    { sha: commit.sha, force: true },
    "PATCH",
  );

  return { url: repo.html_url, fullName: repo.full_name };
}
