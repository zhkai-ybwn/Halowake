module.exports = {
  root: true,
  ignoreFiles: [
    "dist/**/*",
    "src-tauri/target/**/*"
  ],
  extends: [
    "stylelint-config-standard",
    "stylelint-config-prettier"
  ],
  plugins: ["stylelint-order"],
  overrides: [
    {
      files: ["**/*.vue"],
      customSyntax: "postcss-html"
    },
    {
      files: ["**/*.{scss,sass}"],
      customSyntax: "postcss-scss",
      rules: {
        "at-rule-no-unknown": [
          true,
          {
            ignoreAtRules: [
              "each",
              "else",
              "for",
              "forward",
              "function",
              "if",
              "include",
              "mixin",
              "return",
              "use"
            ]
          }
        ],
        "no-invalid-double-slash-comments": null
      }
    }
  ],
  rules: {
    "alpha-value-notation": null,
    "color-function-notation": null,
    "color-hex-length": null,
    "custom-property-empty-line-before": null,
    "declaration-empty-line-before": null,
    "declaration-block-single-line-max-declarations": null,
    "no-descending-specificity": null,
    "no-duplicate-selectors": null,
    "rule-empty-line-before": null,
    "selector-class-pattern": null,
    "selector-not-notation": null,
    "selector-pseudo-class-no-unknown": [true, { ignorePseudoClasses: ["deep", "global", "slotted"] }],
    "value-keyword-case": null,
    "value-no-vendor-prefix": null
  }
};
