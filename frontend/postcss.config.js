export default {
  plugins: {
    "postcss-import": {
      filter: (path) =>
        path !== "tailwindcss" && !path.startsWith("tailwindcss/")
    },
    "@tailwindcss/postcss": {}
  }
};
