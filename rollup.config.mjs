import rust from '@wasm-tool/rollup-plugin-rust';
import url from '@rollup/plugin-url';
import replace from '@rollup/plugin-replace';

const production = !process.env.ROLLUP_WATCH;
const deploy = !!process.env.DEPLOY;
const BASE_URL = process.env.BASE_URL ? `'${process.env.BASE_URL}'` : `'http://localhost:3883'`;

export default {
    input: "./js/main.js",
    output: {
        dir: 'dist/js/',
    },
    plugins:[
        replace({
            BASE_URL,
            preventAssignment: true,
        }),
        rust({
            serverPath: deploy ? "/asteroid-colonies/js/" : "./js/",
        }),
		url(),
    ]
}
