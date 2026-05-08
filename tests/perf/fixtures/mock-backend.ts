/**
 * Mock axhub backend HTTP fixture for Phase 0 walltime tests.
 * Spec: .plan/deploy-time-reduction/phase-0-measurement-first.md §7
 *
 * Latency is configurable per-instance or via MOCK_BACKEND_LATENCY_MS env.
 * Routes only exist for shapes the deploy walltime suite needs; everything
 * else returns 404 so tests detect unexpected calls.
 */

export interface MockBackendStartResult {
  port: number;
  baseUrl: string;
}

const DEFAULT_LATENCY_MS = 5000;

function readEnvLatency(): number | null {
  const raw = process.env.MOCK_BACKEND_LATENCY_MS;
  if (!raw) return null;
  const n = Number.parseInt(raw, 10);
  return Number.isFinite(n) && n >= 0 ? n : null;
}

function delay(ms: number): Promise<void> {
  if (ms <= 0) return Promise.resolve();
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function buildSseStream(): ReadableStream<Uint8Array> {
  const events = [
    'event: status\ndata: {"phase":"building"}\n\n',
    'event: status\ndata: {"phase":"built"}\n\n',
    'event: status\ndata: {"phase":"deploying"}\n\n',
    'event: status\ndata: {"phase":"complete","exit_code":0}\n\n',
  ];
  const encoder = new TextEncoder();
  let i = 0;
  let tick: ReturnType<typeof setInterval> | undefined;
  return new ReadableStream({
    start(controller) {
      tick = setInterval(() => {
        try {
          if (i >= events.length) {
            if (tick) clearInterval(tick);
            controller.close();
            return;
          }
          controller.enqueue(encoder.encode(events[i++]));
        } catch {
          // Controller already closed by client cancel — stop the producer.
          if (tick) clearInterval(tick);
        }
      }, 250);
    },
    cancel() {
      if (tick) clearInterval(tick);
    },
  });
}

export class MockBackendServer {
  private server: ReturnType<typeof Bun.serve> | null = null;
  readonly latencyMs: number;

  constructor(latencyMs?: number) {
    const envLatency = readEnvLatency();
    this.latencyMs = latencyMs ?? envLatency ?? DEFAULT_LATENCY_MS;
  }

  async start(): Promise<MockBackendStartResult> {
    if (this.server) {
      throw new Error("MockBackendServer already running");
    }
    const latency = this.latencyMs;
    const server = Bun.serve({
      port: 0,
      hostname: "127.0.0.1",
      fetch: async (req) => {
        await delay(latency);
        const url = new URL(req.url);
        const path = url.pathname;
        const method = req.method.toUpperCase();

        if (method === "POST" && path === "/api/v1/resolve") {
          return jsonResponse(200, {
            app_id: "paydrop",
            app_slug: "paydrop",
            branch: "main",
            commit_sha: "abc1234567890",
            commit_message: "test commit",
            eta_sec: 60,
          });
        }
        if (method === "POST" && path === "/api/v1/apps") {
          return jsonResponse(201, {
            app_id: "new-app-12345",
            app_slug: "new-app",
            subdomain: "new-app.staging.axhub.dev",
          });
        }
        if (method === "POST" && path === "/api/v1/deploys") {
          return jsonResponse(201, {
            deploy_id: "deploy-xyz789",
            app_id: "paydrop",
            status: "queued",
            eta_sec: 300,
          });
        }
        if (
          method === "GET" &&
          /^\/api\/v1\/deploys\/[^/]+\/status\/?$/.test(path)
        ) {
          return new Response(buildSseStream(), {
            status: 200,
            headers: {
              "content-type": "text/event-stream",
              "cache-control": "no-cache",
            },
          });
        }
        return new Response("not found", { status: 404 });
      },
    });
    this.server = server;
    const port = (server as unknown as { port: number }).port;
    return { port, baseUrl: `http://127.0.0.1:${port}` };
  }

  async stop(): Promise<void> {
    if (!this.server) return;
    await this.server.stop(true);
    this.server = null;
  }
}
