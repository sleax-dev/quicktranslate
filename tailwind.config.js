/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,vue}"],
  theme: {
    extend: {
      // Inter ships with the app (@fontsource-variable/inter), so it needs no network.
      fontFamily: { sans: ['"Inter Variable"', "system-ui", "sans-serif"] },
    },
  },
}
