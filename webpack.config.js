const path = require("path");

function resolve(dir) {
  return path.join(__dirname, dir)
}

module.exports = {
  entry: {
    index: resolve("src/js/wrapper.js"),
  },
  output: {
    path: resolve(".build/assets/scripts"),
  },
  mode: "development",
  devServer: {
    publicPath: "/",
    contentBase: "./.build",
    hot: true
  }
}
