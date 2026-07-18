import { spawn } from "node:child_process";
import { access, mkdtemp, rm } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";

export function startVite(repoRoot, port) {
  const child = spawn(
    process.execPath,
    [
      "node_modules/vite/bin/vite.js",
      "--host",
      "127.0.0.1",
      "--port",
      String(port),
      "--strictPort",
    ],
    {
      cwd: repoRoot,
      env: { ...process.env, BROWSER: "none" },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  child.stdout.on("data", (chunk) => process.stdout.write(chunk));
  child.stderr.on("data", (chunk) => process.stderr.write(chunk));
  return child;
}

export function startChrome(chromePath, debugPort, dataDir) {
  const debug = process.env.HEADLESS_CHROME_DEBUG === "1";
  const child = spawn(
    chromePath,
    [
      "--headless=new",
      "--disable-background-networking",
      "--disable-component-update",
      "--disable-default-apps",
      "--disable-gpu",
      "--disable-dev-shm-usage",
      "--disable-sync",
      "--hide-scrollbars",
      "--metrics-recording-only",
      "--no-first-run",
      "--no-default-browser-check",
      "--no-pings",
      `--remote-debugging-port=${debugPort}`,
      `--user-data-dir=${dataDir}`,
      "about:blank",
    ],
    {
      stdio: ["ignore", "ignore", debug ? "pipe" : "ignore"],
    },
  );
  child.stderr?.on("data", (chunk) => process.stderr.write(chunk));
  return child;
}

export async function createCdpClient(wsUrl) {
  const socket = new WebSocket(wsUrl);
  const callbacks = new Map();
  let nextId = 1;

  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });

  socket.addEventListener("message", (event) => {
    const message = JSON.parse(String(event.data));
    if (!message.id) {
      return;
    }
    const callback = callbacks.get(message.id);
    if (!callback) {
      return;
    }
    callbacks.delete(message.id);
    if (message.error) {
      callback.reject(new Error(message.error.message));
      return;
    }
    callback.resolve(message.result ?? {});
  });

  return {
    send(method, params = {}, sessionId) {
      const id = nextId++;
      const message = { id, method, params };
      if (sessionId) {
        message.sessionId = sessionId;
      }
      socket.send(JSON.stringify(message));
      return new Promise((resolve, reject) => {
        callbacks.set(id, { resolve, reject });
      });
    },
    close() {
      socket.close();
    },
  };
}

export async function waitForExpression(
  client,
  sessionId,
  expression,
  timeoutMs = 10000,
) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  let lastValue;
  while (Date.now() < deadline) {
    try {
      const result = await client.send(
        "Runtime.evaluate",
        {
          expression,
          awaitPromise: true,
          returnByValue: true,
        },
        sessionId,
      );
      if (result.exceptionDetails) {
        const description = result.exceptionDetails.exception?.description
          ?? result.exceptionDetails.text
          ?? "Unknown page exception";
        throw new Error(description);
      }
      lastValue = result.result?.value;
      if (lastValue) {
        return;
      }
    } catch (error) {
      lastError = error;
    }
    await sleep(100);
  }
  throw new Error(
    `Timed out waiting for page expression. lastValue=${String(lastValue)} ${lastError ?? ""}`,
  );
}

export async function waitForChromeWebSocket(port) {
  const endpoint = `http://127.0.0.1:${port}/json/version`;
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(endpoint);
      if (response.ok) {
        const version = await response.json();
        return version.webSocketDebuggerUrl;
      }
    } catch {
      // Keep polling until Chrome exposes the debugging endpoint.
    }
    await sleep(200);
  }
  throw new Error("Chrome DevTools endpoint did not become available.");
}

export async function waitForHttp(url) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {
      // Keep polling until Vite is ready.
    }
    await sleep(200);
  }
  throw new Error(`Vite server did not become available: ${url}`);
}

export function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : null;
      server.close(() => {
        if (port) {
          resolve(port);
          return;
        }
        reject(new Error("Could not allocate a free port."));
      });
    });
    server.on("error", reject);
  });
}

export function makeTempDir(prefix) {
  return mkdtemp(path.join(os.tmpdir(), prefix));
}

export async function resolveChromePath() {
  if (process.env.CHROME_PATH) {
    return process.env.CHROME_PATH;
  }

  const candidates = [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
  ];

  for (const candidate of candidates) {
    if (await fileExists(candidate)) {
      return candidate;
    }
  }

  const pathCandidate = await findExecutableOnPath([
    "google-chrome",
    "chrome",
    "chromium",
    "chromium-browser",
  ]);
  if (pathCandidate) {
    return pathCandidate;
  }

  throw new Error(
    "Chrome executable was not found. Set CHROME_PATH to run browser checks.",
  );
}

export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function waitForProcessExit(child, timeoutMs) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve();
  }

  return new Promise((resolve) => {
    const timeout = setTimeout(resolve, timeoutMs);
    child.once("exit", () => {
      clearTimeout(timeout);
      resolve();
    });
  });
}

export async function rmWithRetry(targetPath, attempts = 5) {
  let lastError;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      await rm(targetPath, { recursive: true, force: true });
      return;
    } catch (error) {
      lastError = error;
      await sleep(200 * attempt);
    }
  }
  throw lastError;
}

async function findExecutableOnPath(names) {
  const pathDirs = (process.env.PATH ?? "").split(path.delimiter).filter(Boolean);
  const extensions =
    process.platform === "win32"
      ? (process.env.PATHEXT ?? ".EXE;.CMD;.BAT;.COM").split(";")
      : [""];

  for (const dir of pathDirs) {
    for (const name of names) {
      for (const extension of extensions) {
        const candidate = path.join(dir, `${name}${extension}`);
        if (await fileExists(candidate)) {
          return candidate;
        }
      }
    }
  }
  return null;
}

async function fileExists(candidate) {
  try {
    await access(candidate);
    return true;
  } catch {
    return false;
  }
}
