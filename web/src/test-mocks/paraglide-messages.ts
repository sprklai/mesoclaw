// Stub for $lib/paraglide/messages — returns the key as-is in test environments.
// The real generated file is not available in the worktree.
// Using Proxy to handle any named export dynamically.
const handler: ProxyHandler<Record<string, unknown>> = {
  get(_target, key: string) {
    return () => key;
  },
};

export default new Proxy({}, handler);

// Re-export via Proxy so `import * as m` works
module.exports = new Proxy({}, handler);
