import {mkdir,readdir,copyFile,writeFile,readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {createReadStream} from 'node:fs';
import {join,resolve} from 'node:path';
const root=resolve('.'),out=join(root,'release-assets');await mkdir(out,{recursive:true});
const platform=process.platform==='win32'?'Windows':'Linux';
const inputs=platform==='Windows'?['src-tauri/target/release/bundle/nsis','store-output']:['src-tauri/target/release/bundle/deb','src-tauri/target/release/bundle/appimage'];
const sums=[];
for(const directory of inputs){for(const name of await readdir(join(root,directory))){if(!/\.(exe|msix|deb|AppImage)$/.test(name))continue;const source=join(root,directory,name);await copyFile(source,join(out,name));const hash=createHash('sha256');for await(const chunk of createReadStream(source))hash.update(chunk);sums.push(`${hash.digest('hex')}  ${name}`);}}
if(!sums.length)throw new Error('No release packages found');
await writeFile(join(out,`SHA256SUMS-${platform}.txt`),sums.join('\n')+'\n');
for(const name of ['tool-versions.json','translation-versions.json','THIRD-PARTY-NOTICES.txt','DEPENDENCY-LICENSES.txt']){await copyFile(join(root,'src-tauri/binaries',name),join(out,`${platform}-${name}`));}
console.log(`${platform}: ${sums.length} packages staged.`);
