import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { resolve, dirname, join } from 'node:path';
import { mkdir, readFile } from 'node:fs/promises';
const root=resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output=join(root,'test-results');await mkdir(output,{recursive:true});
const vite=spawn(process.execPath,[join(root,'node_modules/vite/bin/vite.js'),'--host','127.0.0.1','--port','1427','--strictPort'],{cwd:root,stdio:'ignore'});
let ready=false;
for(let i=0;i<100;i++){try{if((await fetch('http://127.0.0.1:1427')).ok){ready=true;break;}}catch{}await new Promise(r=>setTimeout(r,100));}
if(!ready){vite.kill();throw new Error('UI test server did not start');}

const browser = await chromium.launch({executablePath:process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH || undefined, headless:true, args:['--no-sandbox']});
const page = await browser.newPage({viewport:{width:1280,height:850}});
const pageErrors=[];page.on('pageerror',e=>pageErrors.push(e.message));
await page.route('**/qa-fixture.mp4',route=>route.fulfill({path:join(root,'tests/fixtures/speech.mp4'),contentType:'video/mp4'}));
const deviceOverride=process.env.VIDEXTRACT_DEVICE_PREVIEW?JSON.parse(await readFile(process.env.VIDEXTRACT_DEVICE_PREVIEW,'utf8')):null;
await page.addInitScript((deviceOverride) => {
  window.isTauri=true;
  localStorage.setItem("ve-locale","tr");
  window.__qaCalls=[];
  const deps={ytDlp:true,ffmpeg:true,ffprobe:true,ytDlpVersion:'test',defaultDownloadDir:'/tmp/downloads',whisper:true,modelReady:false,
    device:{cpuName:'QA processor',physicalCores:4,logicalCores:8,totalMemory:8589934592,availableMemory:4294967296,gpus:[{name:'QA graphics',dedicatedMemory:2147483648}],tier:'balanced',recommendedModel:'base',threads:3,gpuAcceleration:false},
    models:[{id:'tiny',bytes:77691713,installed:false,minimumMemory:734003200},{id:'base',bytes:147951465,installed:false,minimumMemory:1153433600},{id:'small',bytes:487601967,installed:false,minimumMemory:2306867200}]};
  if(deviceOverride)deps.device=deviceOverride;
  const transcript={id:'dbe420cc-387d-4c53-8247-5e55d63ed9b9',mediaPath:'/tmp/speech.mp4',mediaTitle:'speech.mp4',createdAt:1792926000,language:'tr',segments:[{start:0,end:3,text:'İLERİ seviyede görüntü işleme.'},{start:3,end:6,text:'Öğrenen modeller ile uygulamalar geliştirelim.'},{start:6,end:10,text:'Yapay zekâ mühendisliği.'}]};
  let library=[];let pending;let translationInstalled=false;let translationEmail="";const translations={};
  window.__TAURI_INTERNALS__={
    metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},
    transformCallback:()=>Math.random()*100000|0,unregisterCallback:()=>{},
    convertFileSrc:()=>'/qa-fixture.mp4',
    invoke:async(cmd,args={})=>{
      window.__qaCalls.push({cmd,args:JSON.parse(JSON.stringify(args))});
      switch(cmd){
        case 'check_dependencies':return structuredClone(deps);
        case 'inspect_device':return structuredClone(deps.device);
        case 'list_transcripts':return library;
        case 'translation_status':return {email:translationEmail,dailyLimit:translationEmail?50000:5000,offlineInstalled:translationInstalled,offlineSupported:true,offlineBytes:124742526,offlineEngine:true};
        case 'save_translation_settings':translationEmail=args.email;return;
        case 'cached_translation':return translations[args.id+args.targetLanguage+(args.translationMode||"online")]??null;
        case 'install_translation_model':translationInstalled=true;return;
        case 'translate_transcript':{
          args.onEvent.onmessage({phase:'translating',progress:25,message:''});
          if(window.__qaCancelTranslation){await new Promise((resolve,reject)=>{pending=reject});return;}
          await new Promise(r=>setTimeout(r,200));
          const result={transcriptId:args.id,targetLanguage:args.targetLanguage,segments:transcript.segments.map((s,i)=>({...s,text:['Advanced image processing.','Let us build applications with learning models.','Artificial intelligence engineering.'][i]}))};
          translations[args.id+args.targetLanguage+(args.translationMode||"online")]=result;return result;
        }
        case 'export_translation':return;

        case 'plugin:dialog|open':return '/tmp/speech.mp4';
        case 'plugin:dialog|save':return '/tmp/export.'+(args.options?.filters?.[0]?.extensions?.[0]||'txt');
        case 'plugin:clipboard-manager|read_text':return '';
        case 'prepare_media':return {path:args.path,title:'speech.mp4',duration:11,playbackUrl:'/qa-fixture.mp4'};
        case 'install_model':args.onEvent.onmessage({phase:'model',progress:50,message:'Model indiriliyor'});await new Promise(r=>setTimeout(r,180));deps.modelReady=true;deps.models.find(model=>model.id===(args.modelId||'base')).installed=true;return;
        case 'transcribe_media':
          args.onEvent.onmessage({phase:'transcribing',progress:35,message:'Konuşmalar metne dönüştürülüyor'});
          if(window.__qaCancel){await new Promise((resolve,reject)=>{pending=reject});return;}
          await new Promise(r=>setTimeout(r,200));library=[transcript];return transcript;
        case 'cancel_download':if(pending){pending('İptal edildi');pending=null;}return;
        case 'export_transcript':return;
        case 'extract_clip':await new Promise(r=>setTimeout(r,100));return;
        case 'analyze_url':return {id:'qa',title:'QA video',uploader:'Test',duration:11,thumbnail:null,webpageUrl:'https://example.org/test',extractor:'generic',viewCount:10,isLive:false};
        case 'start_download':args.onEvent.onmessage({kind:'completed',jobId:args.request.jobId,progress:100,message:'Tamamlandı',outputPath:'/tmp/speech.mp4',speed:null,eta:null});return;
        default:return;
      }
    }
  };
},deviceOverride);
try {
  await page.goto('http://127.0.0.1:1427');
  await page.getByRole('button',{name:'Video stüdyosu',exact:true}).click();
  await page.getByRole('heading',{name:'Videonun içinde ara.'}).waitFor();
  await page.waitForTimeout(350);
  await page.screenshot({path:join(output,'studio-empty.png')});
  await page.getByRole('button',{name:'Modeli indir',exact:true}).click();
  await page.getByText('Model hazır. Sonraki transkriptler çevrimdışı çalışır.').waitFor();
  await page.getByRole('button',{name:'Video veya ses aç',exact:true}).click();
  await page.getByRole('heading',{name:'speech.mp4',exact:true}).waitFor();
  await page.getByRole('button',{name:'Yazıya dök',exact:true}).click();
  await page.getByText('Transkript hazır. Bir cümleye tıklayarak o ana gidebilirsin.').waitFor();
  await page.locator('.transcript-toolbar').getByText('Konuşma dili: Türkçe',{exact:false}).waitFor();
  for (const [mode,label] of [['gpu','Ekran kartı · daha hızlı işleme'],['cpu','İşlemci · standart işleme'],['auto','Otomatik · önerilen']]) {
    await page.getByRole('button',{name:'İşleme tercihi',exact:true}).click();
    await page.getByRole('option').filter({hasText:label}).click();
    await page.getByRole('button',{name:'Yeniden yazıya dök',exact:true}).click();
    await page.getByText('Transkript hazır. Bir cümleye tıklayarak o ana gidebilirsin.',{exact:true}).waitFor();
    assert.equal(await page.evaluate(()=>window.__qaCalls.filter(c=>c.cmd==='transcribe_media').at(-1).args.processingMode),mode);
  }
  assert.equal(await page.locator('.transcript-segment').count(),3);
  await page.getByRole('textbox',{name:'Transkript içinde ara'}).fill('ileri');
  assert.equal(await page.locator('.transcript-segment').count(),1);
  assert.equal(await page.locator('mark').innerText(),'İLERİ');
  await page.locator('.transcript-segment').click();
  await page.getByRole('button',{name:'Aramayı temizle',exact:true}).click();
  await page.locator('.transcript-segment').nth(1).click();
  await page.waitForFunction(()=>document.querySelector('video').currentTime>=3,{},{timeout:5000});
  await page.getByRole('button',{name:'SRT',exact:true}).click();
  await page.getByText('SRT dosyası kaydedildi.').waitFor();
  const exportCall=await page.evaluate(()=>window.__qaCalls.find(c=>c.cmd==='export_transcript'));
  assert.equal(exportCall.args.format,'srt');
  await page.locator('.translation-heading button').click();
  await page.getByRole('button',{name:/^Çevrimdışı/}).click();
  await page.getByRole('button',{name:'Dil paketini indir',exact:true}).click();
  await page.locator('.translation-start').waitFor();
  await page.locator('.translation-start').click();
  await page.getByText('Advanced image processing.',{exact:true}).waitFor();
  assert.equal(await page.locator('.bilingual-text').count(),3);
  assert.equal(await page.locator('.translated-text').first().getAttribute('lang'),'en');
  await page.locator('.translation-exports').getByRole('button',{name:'SRT',exact:true}).click();
  const translatedExport=await page.evaluate(()=>window.__qaCalls.filter(c=>c.cmd==='export_translation').at(-1));
  assert.equal(translatedExport.args.targetLanguage,'en');
  assert.equal(translatedExport.args.format,'srt');
  assert.equal(await page.evaluate(()=>window.__qaCalls.filter(c=>c.cmd==='translate_transcript').at(-1).args.translationMode),'offline');
  const callsBeforeCache=await page.evaluate(()=>window.__qaCalls.filter(c=>c.cmd==='translate_transcript').length);
  await page.getByRole('button',{name:'Çeviri dili',exact:true}).click();
  await page.getByRole('option',{name:'Deutsch',exact:true}).click();
  await page.waitForFunction(()=>document.querySelectorAll('.translated-text').length===0);
  await page.getByRole('button',{name:'Çeviri dili',exact:true}).click();
  await page.getByRole('option',{name:'English',exact:true}).click();
  await page.getByText('Advanced image processing.',{exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>window.__qaCalls.filter(c=>c.cmd==='translate_transcript').length),callsBeforeCache);
  await page.getByRole('button',{name:/^Çevrimiçi/}).click();
  await page.waitForFunction(()=>document.querySelectorAll('.translated-text').length===0);
  await page.locator('.translation-start').click();
  await page.getByText('Advanced image processing.',{exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>window.__qaCalls.filter(c=>c.cmd==='translate_transcript').at(-1).args.translationMode),'online');
  await page.locator('.translation-email summary').click();
  await page.locator('.translation-email input').fill('tester@example.org');
  await page.locator('.translation-email').getByRole('button',{name:'Kaydet',exact:true}).click();
  await page.getByText('Ücretsiz günlük sınır: 50.000 karakter.',{exact:true}).waitFor();
  await page.getByRole('button',{name:'MP4 kesiti kaydet',exact:true}).click();
  await page.getByText('Video kesiti kaydedildi.').waitFor();
  await page.locator('.main-content').evaluate(el=>el.scrollTop=0);
  await page.screenshot({path:join(output,'studio-transcript.png')});
  await page.getByRole('textbox',{name:'Transkript içinde ara'}).fill('bulunmayan');
  await page.getByText('Bu ifadeyi bulamadık.').waitFor();
  await page.getByRole('button',{name:'Tüm transkripti göster',exact:true}).click();
  await page.evaluate(()=>window.__qaCancel=true);
  await page.getByRole('button',{name:'Yeniden yazıya dök',exact:true}).click();
  await page.getByRole('button',{name:'İptal',exact:true}).click();
  await page.getByRole('alert').filter({hasText:'İptal edildi'}).waitFor();
  assert.equal(await page.locator('.transcript-segment').count(),3);
  await page.getByRole('button',{name:'Ana sayfa',exact:true}).click();
  await page.getByPlaceholder('Video veya oynatma listesi bağlantısını yapıştır...').fill('https://example.org/test');
  await page.getByRole('button',{name:'Bağlantıyı analiz et',exact:true}).click();
  await page.getByRole('heading',{name:'QA video',exact:true}).waitFor();
  await page.getByRole('button',{name:/İndirmeyi başlat/}).click();
  await page.getByRole('button',{name:'Geçmiş',exact:true}).click();
  await page.getByRole('button',{name:'Stüdyoda aç',exact:true}).click();
  await page.getByRole('heading',{name:'Video stüdyosu',exact:true}).waitFor();
  await page.getByRole('button',{name:'Ayarlar',exact:true}).click();
  await page.locator('.theme-picker').getByRole('button',{name:'Açık',exact:true}).click();
  await page.getByRole('button',{name:'Video stüdyosu',exact:true}).click();
  await page.screenshot({path:join(output,'studio-light.png')});
  await page.setViewportSize({width:900,height:620});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>window.innerWidth),false);
  await page.screenshot({path:join(output,'studio-small.png')});
  await page.setViewportSize({width:1360,height:940});
  await page.evaluate(()=>{window.__qaCancel=false;localStorage.setItem('ve-motion','true');});
  const nativeNames={tr:'Türkçe',en:'English',es:'Español',fr:'Français',de:'Deutsch',pt:'Português',ru:'Русский',ar:'العربية',hi:'हिन्दी',zh:'中文'};
  let previous='tr';
  for(const [code,nativeName] of Object.entries(nativeNames)){
    const before=JSON.parse(await readFile(join(root,`src/i18n/${previous}.json`),'utf8'));
    const dict=JSON.parse(await readFile(join(root,`src/i18n/${code}.json`),'utf8'));
    await page.locator('.header-actions').getByRole('button',{name:before.interfaceLanguage,exact:true}).click();
    await page.getByRole('option',{name:nativeName,exact:true}).click();
    await page.waitForFunction(code=>document.documentElement.lang===code,code);
    assert.equal(await page.evaluate(()=>document.documentElement.dir),code==='ar'?'rtl':'ltr');
    await page.locator('[data-testid="nav-home"]').click();
    await page.getByRole('heading',{name:dict.heroTitle,exact:true}).waitFor();
    await page.waitForTimeout(320);
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>window.innerWidth),false,code);
    if(['tr','en','ar','hi','zh'].includes(code))await page.screenshot({path:join(output,`home-${code}.png`)});
    await page.locator('[data-testid="nav-studio"]').click();
    await page.getByRole('button',{name:dict.speechLanguage,exact:true}).click();
    await page.getByRole('option',{name:nativeName,exact:true}).click();
    await page.getByRole('button',{name:dict.retranscribe,exact:true}).click();
    await page.getByText(dict.transcriptDone,{exact:true}).waitFor();
    assert.equal(await page.evaluate(()=>window.__qaCalls.filter(c=>c.cmd==='transcribe_media').at(-1).args.language),code);
    assert.equal(await page.evaluate(()=>document.documentElement.lang),code);
    previous=code;
  }
  // Switch interface to Turkish while speech remains Chinese, then inspect the custom menu.
  const zh=JSON.parse(await readFile(join(root,'src/i18n/zh.json'),'utf8'));
  await page.locator('.header-actions').getByRole('button',{name:zh.interfaceLanguage,exact:true}).click();
  await page.getByRole('option',{name:'Türkçe',exact:true}).click();
  await page.getByRole('button',{name:'Konuşma dili',exact:true}).waitFor();
  assert.equal(await page.getByRole('button',{name:'Konuşma dili',exact:true}).innerText(),'中文');
  await page.locator('.header-actions .theme-button').click(); // return to dark
  await page.getByRole('button',{name:'Konuşma dili',exact:true}).click();
  assert.equal(await page.getByRole('listbox').evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(18, 21, 28)');
  await page.waitForTimeout(350);
  await page.screenshot({path:join(output,'language-menu-dark.png')});
  await page.keyboard.press('Escape');
  assert.equal(await page.getByRole('listbox').count(),0);
  await page.emulateMedia({reducedMotion:'reduce'});
  assert.equal(await page.locator('.studio').evaluate(el=>parseFloat(getComputedStyle(el).animationDuration)<.01),true);
  await page.emulateMedia({reducedMotion:'no-preference'});
  await page.getByRole('button',{name:'Ayarlar',exact:true}).click();
  await page.getByRole('switch',{name:'Hareket ve efektler',exact:true}).click();
  assert.equal(await page.evaluate(()=>document.documentElement.dataset.motion),'off');
  await page.waitForTimeout(300);await page.screenshot({path:join(output,'settings-device.png')});
  await page.getByRole('button',{name:'Neden VidExtract?',exact:true}).click();
  await page.getByRole('heading',{name:'Neden VidExtract?',level:2,exact:true}).waitFor();
  await page.screenshot({path:join(output,'why-vidextract.png')});
  assert.equal(await page.locator('.value-card').count(),4);
  await page.locator('[data-testid="nav-home"]').click();
  await page.locator('.url-field input').fill('');
  if(await page.locator('.notice:visible .icon-button').count())await page.locator('.notice:visible .icon-button').first().click();
  await page.waitForTimeout(350);await page.screenshot({path:join(output,'home-tr-dark.png')});
  await page.locator('.header-actions .theme-button').click();
  await page.waitForTimeout(350);await page.screenshot({path:join(output,'home-tr-light.png')});
  assert.deepEqual(pageErrors,[]);
  console.log('UI PASS: downloads, local transcription, search/seek/export/clip/cancel; all 10 interface and speech languages; RTL, custom dark menus, independent language choices, reduced motion, hardware and value proposition.');
} catch (e) {console.log('Media diagnostics:',await page.locator('video').evaluate(v=>({time:v.currentTime,ready:v.readyState,src:v.currentSrc,error:v.error?.message})));await page.screenshot({path:join(output,'failure.png')});throw e;} finally {await browser.close();vite.kill();}
