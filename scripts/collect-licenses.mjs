import {readdir,readFile,writeFile,mkdir} from 'node:fs/promises';
import {dirname,join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const output=join(root,'src-tauri/binaries');await mkdir(output,{recursive:true});
const blocks=[];
for(const name of ['LICENSE','THIRD_PARTY.md','PRIVACY.md']){try{await writeFile(join(output,name),await readFile(join(root,name)));}catch(error){throw new Error(`${name}: ${error.message}`);}}
async function licenses(directory,label){
 const files=await readdir(directory).catch(()=>[]);
 for(const file of files.filter(n=>/^(licen[cs]e|copying|notice|ofl)([._-]|$)/i.test(n))){try{const content=await readFile(join(directory,file),'utf8');if(content.length<200000)blocks.push(`\n========== ${label}: ${file} ==========\n${content}`);}catch{}}
}
const seen=new Set();
async function nodePackage(name,parent=root){
 let location=join(parent,'node_modules',name);
 try{await readFile(join(location,'package.json'));}catch{location=join(root,'node_modules',name);}
 if(seen.has(location))return;seen.add(location);
 const pkg=JSON.parse(await readFile(join(location,'package.json'),'utf8'));
 blocks.push(`\n${pkg.name}@${pkg.version} — ${typeof pkg.license==='string'?pkg.license:'see upstream license'}`);
 await licenses(location,`${pkg.name}@${pkg.version}`);
 for(const dependency of Object.keys(pkg.dependencies??{}))await nodePackage(dependency,location);
}
const pkg=JSON.parse(await readFile(join(root,'package.json'),'utf8'));
for(const name of Object.keys(pkg.dependencies))await nodePackage(name);
const metadata=spawnSync('cargo',['metadata','--manifest-path','src-tauri/Cargo.toml','--format-version','1','--locked'],{cwd:root,encoding:'utf8',maxBuffer:32*1024*1024});
if(metadata.status!==0)throw new Error('Cargo license metadata could not be generated.');
for(const pkg of JSON.parse(metadata.stdout).packages){if(!pkg.source)continue;blocks.push(`\n${pkg.name}@${pkg.version} — ${pkg.license??'see upstream'}`);await licenses(dirname(pkg.manifest_path),`${pkg.name}@${pkg.version}`);}
await writeFile(join(output,'DEPENDENCY-LICENSES.txt'),blocks.join('\n'));
console.log(`Dependency notices collected (${seen.size} JavaScript packages plus Cargo dependencies).`);
