// Config mínima ESLint v9 (flat). Los .svelte los cubre svelte-check.
const browserGlobals = {
	window: "readonly",
	document: "readonly",
	navigator: "readonly",
	location: "readonly",
	fetch: "readonly",
	Headers: "readonly",
	FormData: "readonly",
	TextEncoder: "readonly",
	TextDecoder: "readonly",
	crypto: "readonly",
	btoa: "readonly",
	atob: "readonly",
	CustomEvent: "readonly",
	localStorage: "readonly",
	sessionStorage: "readonly",
	console: "readonly",
	setTimeout: "readonly",
	clearTimeout: "readonly",
	setInterval: "readonly",
	clearInterval: "readonly",
	requestAnimationFrame: "readonly",
	cancelAnimationFrame: "readonly",
};

export default [
	{ ignores: ["node_modules/**", "dist/**", "public/**"] },
	{
		files: ["**/*.js"],
		languageOptions: {
			ecmaVersion: "latest",
			sourceType: "module",
			globals: browserGlobals,
		},
		rules: {
			"no-unused-vars": ["error", { args: "none", caughtErrors: "none" }],
			"no-undef": "error",
		},
	},
	{
		files: ["**/*.svelte.js"],
		languageOptions: {
			ecmaVersion: "latest",
			sourceType: "module",
			globals: {
				...browserGlobals,
				$state: "readonly",
				$derived: "readonly",
				$effect: "readonly",
				$props: "readonly",
			},
		},
	},
	{
		files: ["**/*.test.js"],
		languageOptions: {
			ecmaVersion: "latest",
			sourceType: "module",
			globals: {
				...browserGlobals,
				global: "writable",
				process: "readonly",
				describe: "readonly",
				it: "readonly",
				expect: "readonly",
				beforeEach: "readonly",
				afterEach: "readonly",
				vi: "readonly",
			},
		},
	},
];
