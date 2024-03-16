import svelte from 'rollup-plugin-svelte';
import css from 'rollup-plugin-css-only';
import rust from '@wasm-tool/rollup-plugin-rust';
import url from '@rollup/plugin-url';
import replace from '@rollup/plugin-replace';
import resolve from '@rollup/plugin-node-resolve';

const production = !process.env.ROLLUP_WATCH;
const deploy = !!process.env.DEPLOY;
const BASE_URL = process.env.BASE_URL ? `'${process.env.BASE_URL}'` : `'http://localhost:3883'`;
const SERVER_SYNC = process.env.SERVER_SYNC ?? `false`;
const SYNC_PERIOD = process.env.SYNC_PERIOD ?? `100`;

export default {
    input: "./js/main.js",
    output: {
        sourcemap: true,
        dir: 'dist/js/',
    },
    plugins:[
        replace({
            BASE_URL,
            SERVER_SYNC,
            SYNC_PERIOD,
            preventAssignment: true,
        }),
		svelte({
			compilerOptions: {
				// enable run-time checks when not in production
				dev: !production
			}
		}),
        css({ output: 'bundle.css' }),
        rust({
            serverPath: deploy ? "/asteroid-colonies/js/" : "./js/",
        }),
		url(),
        resolve({
			browser: true,
			dedupe: ['svelte']
		}),
    ]
}
