const path = require("path");

function resolve(dir) {
  return path.join(__dirname, '..', dir)
}

module.exports = {
  entry: {
    index: resolve("src/js/wrapper.js"),
  },
  output: {
    path: resolve(".build/assets/scripts"),
    publicPath: "assets/scripts/"
  },
  experiments: {
    asyncWebAssembly: true,
    syncWebAssembly: true,
  },
  mode: "production"
}
