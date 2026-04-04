const path = require("path");
const CopyWebpackPlugin = require("copy-webpack-plugin");

module.exports = {
  entry: "./src/index.js",
  output: {
    filename: "bundle.js",
    path: path.resolve(__dirname, "dist"),
    publicPath: "./",
  },
  mode: "development",
  experiments: {
    asyncWebAssembly: true,
  },
  devServer: {
    static: {
      directory: path.resolve(__dirname, "dist"),
    },
    open: true,
    port: 8080,
    headers: {
      // Required for OPFS access in some browsers
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
    proxy: [
      {
        // Proxy GitHub Releases to avoid CORS issues in development
        context: ["/github-releases"],
        target: "https://github.com",
        pathRewrite: { "^/github-releases": "" },
        changeOrigin: true,
        followRedirects: true,
      },
    ],
  },
  plugins: [
    new CopyWebpackPlugin({
      patterns: [
        { from: path.resolve(__dirname, "../pkg/*.wasm"), to: "[name][ext]" },
      ],
    }),
  ],
};
