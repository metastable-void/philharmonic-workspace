const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const CssMinimizerPlugin = require('css-minimizer-webpack-plugin');

module.exports = (env, argv) => {
  const isProduction = argv.mode === 'production';

  return {
    entry: './src/index.tsx',
    output: {
      path: path.resolve(__dirname, '../dist'),
      filename: 'main.js',
      clean: true,
      publicPath: '/',
    },
    optimization: {
      moduleIds: 'deterministic',
      chunkIds: 'deterministic',
      minimizer: ['...', new CssMinimizerPlugin()],
    },
    cache: false,
    devtool: isProduction ? 'source-map' : 'eval-source-map',
    resolve: {
      extensions: ['.tsx', '.ts', '.js'],
    },
    module: {
      rules: [
        {
          test: /\.tsx?$/,
          use: 'ts-loader',
          exclude: /node_modules/,
        },
        {
          test: /\.css$/,
          use: [isProduction ? MiniCssExtractPlugin.loader : 'style-loader', 'css-loader'],
        },
      ],
    },
    plugins: [
      new HtmlWebpackPlugin({
        template: './src/index.html',
        filename: 'index.html',
        favicon: './src/icon.svg',
      }),
      ...(isProduction
        ? [
            new MiniCssExtractPlugin({
              filename: 'main.css',
            }),
          ]
        : []),
    ],
    devServer: {
      port: 8081,
      historyApiFallback: true,
      proxy: [
        {
          context: ['/config', '/sign-in', '/mint-ephemeral', '/version'],
          target: 'http://127.0.0.1:3100',
        },
      ],
    },
  };
};
