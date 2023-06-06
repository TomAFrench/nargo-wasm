import { esbuildPlugin } from "@web/dev-server-esbuild";

export default {
  nodeResolve: true,
  files: ["src/**/*.test.ts", "src/**/*.spec.ts"],
  plugins: [
    esbuildPlugin({
      ts: true,
    }),
  ],
  testFramework: {
    config: {
      ui: "bdd",
      timeout: 40000,
    },
  },
  testRunnerHtml: (testFramework) => `
  <html>
    <head>
      <script type="module" src="${testFramework}"></script>
      <script type="module">import 'jest-browser-globals';</script>
    </head>
  </html>
`,
};
