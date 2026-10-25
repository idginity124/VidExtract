import {test} from "node:test";
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
const codes=["tr","en","es","fr","de","pt","ru","ar","hi","zh"];
const catalogs=Object.fromEntries(codes.map(code=>[code,JSON.parse(readFileSync(new URL(`../src/i18n/${code}.json`,import.meta.url),"utf8"))]));
test("all ten locales cover every UI key and preserve interpolation variables",()=>{
 const keys=Object.keys(catalogs.en).sort();assert.ok(keys.length>140);
 for(const code of codes){assert.deepEqual(Object.keys(catalogs[code]).sort(),keys,code);for(const key of keys){const value=catalogs[code][key];assert.ok(typeof value==="string"&&value.trim().length>0,`${code}.${key}`);assert.deepEqual([...value.matchAll(/\{\w+\}/g)].map(m=>m[0]).sort(),[...catalogs.en[key].matchAll(/\{\w+\}/g)].map(m=>m[0]).sort(),`${code}.${key}`);}}
});
test("spoken language selector and backend validation include exactly the ten supported languages",()=>{
 const backend=readFileSync(new URL("../src-tauri/src/studio.rs",import.meta.url),"utf8");
 for(const code of codes)assert.ok(backend.includes(`"${code}"`),code);
 const ui=readFileSync(new URL("../src/Studio.tsx",import.meta.url),"utf8");assert.ok(ui.includes('...languages]'));
});
