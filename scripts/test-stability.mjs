import {chromium} from 'playwright';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {dirname,join,resolve} from 'node:path';
import assert from 'node:assert/strict';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const server=spawn(process.execPath,[join(root,'node_modules/vite/bin/vite.js'),'--host','127.0.0.1','--port','1428','--strictPort'],{cwd:root,stdio:'ignore'});
let browser;
try{
 for(let i=0;i<80;i++){try{if((await fetch('http://127.0.0.1:1428')).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 browser=await chromium.launch({executablePath:process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH||undefined,args:['--no-sandbox']});
 const page=await browser.newPage({viewport:{width:1280,height:850}});
 await page.addInitScript(()=>{localStorage.setItem('ve-locale','tr');localStorage.setItem('ve-motion','true');window.isTauri=true;window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},invoke:async cmd=>cmd==='check_dependencies'?{ytDlp:true,ffmpeg:true,ffprobe:true,whisper:true,modelReady:true,defaultDownloadDir:'/tmp',ytDlpVersion:'test'}:cmd==='list_transcripts'?[]:null};});
 await page.goto('http://127.0.0.1:1428');
 async function stationary(selector){
  const frames=await page.locator(selector).evaluate(async el=>{const values=[];for(let i=0;i<14;i++){await new Promise(requestAnimationFrame);const r=el.getBoundingClientRect();values.push({x:r.x,y:r.y,width:r.width,transform:getComputedStyle(el).transform,animation:getComputedStyle(el).animationName});}return values;});
  for(const f of frames){assert.ok(Math.abs(f.x-frames[0].x)<.25);assert.ok(Math.abs(f.y-frames[0].y)<.25);assert.ok(Math.abs(f.width-frames[0].width)<.25);assert.equal(f.transform,'none');assert.equal(f.animation,'none');}
 }
 for(let i=0;i<4;i++){
  for(const key of ['home','studio','history']){await page.locator(`[data-testid="nav-${key}"]`).click();await stationary('.page-header');await stationary('.main-content');}
  await page.getByRole('button',{name:'Ayarlar',exact:true}).click();await stationary('.settings-grid');
  await page.getByRole('button',{name:'Neden VidExtract?',exact:true}).click();await stationary('.about-view');
 }
 await page.locator('[data-testid="nav-studio"]').click();
 for(let i=0;i<6;i++){
  const label=i%2?'Konuşma modeli':'Konuşma dili';
  const trigger=page.getByRole('button',{name:label,exact:true});
  await trigger.scrollIntoViewIfNeeded();
  const before=await trigger.boundingBox();await trigger.hover();await page.mouse.down();
  await stationary('.select-trigger:hover');await page.mouse.up();
  const after=await trigger.boundingBox();assert.ok(Math.abs(before.width-after.width)<.25);
  await page.getByRole('listbox').waitFor();await stationary('.select-menu');
  await page.keyboard.press('ArrowDown');await stationary('.select-menu');
  await page.keyboard.press('Escape');assert.equal(await page.getByRole('listbox').count(),0);
 }
 console.log('PASS: 20 tab transitions and 6 popup openings remain stationary across animation frames; keyboard selection/close works.');
}finally{await browser?.close();server.kill();}
