/* VOID CHOIR test suite. Run: npm install && npm test
   Thresholds are calibrated against renders of the ORIGINAL repo code (see CHANGES.md);
   they are regression guards on measurable properties, not proof a kit "sounds good". */
/* T-31..T-36 added by gen-20260923T084400Z-ten-new-styles — see plan/DECISIONS.md D-005 */
const {OfflineAudioContext}=require('node-web-audio-api');
require('../void-choir-engine.js');
const V=globalThis.VoidChoir, D=require('./dsp.js'), fs=require('fs');
const SR=44100, T0=0.02, fOf=m=>440*Math.pow(2,(m-69)/12);
let pass=0,fail=0;
const ok=(n,c,d)=>{c?(pass++,console.log('PASS  '+n)):(fail++,console.log('FAIL  '+n+'  -> '+d));};
const onset=d=>{const pk=D.peakAbs(d);for(let i=0;i<d.length;i++)if(Math.abs(d[i])>=pk*0.05)return i/SR;return Infinity;};
const solo=(res,id)=>{Object.keys(res.channels).forEach(k=>{if(k!==id)res.channels[k].gain.gain.value=0;});res.reverbReturn.gain.value=0;res.channels[id].gain.gain.value=0.9;};

async function drum(voice,style,sec){const c=new OfflineAudioContext(1,Math.ceil(SR*sec),SR),r=V.createGraph(c,style,'t');solo(r,'drums');r.voices[voice](T0,{vel:1,punch:1});return (await c.startRendering()).getChannelData(0);}
async function guitar(midi,sec,drive){const c=new OfflineAudioContext(1,Math.ceil(SR*sec),SR),r=V.createGraph(c,'blackgaze','t');solo(r,'gtrtrem');r.amps.gtrtrem.setDrive(drive);
  V.guitarPick(c,r.amps.gtrtrem,T0,{freq:fOf(midi),dur:sec-0.1,amp:0.25,isLead:true,noise:r.noise});return (await c.startRendering()).getChannelData(0);}
async function bass(midi,sec){const c=new OfflineAudioContext(1,Math.ceil(SR*sec),SR),r=V.createGraph(c,'blackgaze','t');solo(r,'bass');
  V.bassNote(c,r.channels.bass.gain,T0,{midi,dur:0.3,drive:50});return (await c.startRendering()).getChannelData(0);}
async function full(style,seed,sec,o){const c=new OfflineAudioContext(2,Math.ceil(SR*sec),SR),r=V.createGraph(c,style,seed),cp=V.createComposer(seed,style),p=cp.makePatterns(o.intensity);
  let t=0,g=0;while(t<sec){for(const e of cp.eventsForStep(g,p,{intensity:o.intensity,tremPerBeat:o.tpb})){const at=t+(e.offset||0);if(at>sec)continue;const ch=cp.chords[e.cIdx];
    if(e.type==='drum')r.voices[e.voice](at,{vel:e.vel,punch:e.punch});
    else if(e.type==='chord')e.midis.forEach((m,k)=>V.guitarPick(c,r.amps.gtrchord,at+k*0.004,{freq:fOf(m),dur:e.dur,amp:0.45/(k+1.2),isLead:false,noise:r.noise}));
    else if(e.type==='trem')V.guitarPick(c,r.amps.gtrtrem,at+e.jitter,{freq:fOf(e.midi),dur:e.dur,amp:e.amp,isLead:true,noise:r.noise});
    else if(e.type==='bass')V.bassNote(c,r.channels.bass.gain,at,{midi:e.midi,dur:e.dur,drive:100});
    else if(e.type==='pad')V.padChord(r,ch,at,e.dur);
    else if(e.type==='drone')V.droneNote(r,ch,at,e.dur,V.makeRng('d'+g));}
    t+=cp.stepDur;g++;}
  const b=await c.startRendering();return [b.getChannelData(0),b.getChannelData(1)];}

function ihr(d,f0,N=16384){const m=D.spectrum(d,Math.floor(0.25*SR),N);let h=0,ih=0;
  for(let i=Math.floor(150*N/SR);i<Math.floor(9000*N/SR);i++){const f=i*SR/N,k=Math.max(1,Math.round(f/f0)),dev=Math.abs(f-k*f0)/Math.max(0.035*f0,6*SR/N);const e=m[i]*m[i];dev<1?h+=e:ih+=e;}
  return 10*Math.log10((ih+1e-20)/(h+1e-20));}

(async()=>{
 const src=fs.readFileSync(__dirname+'/../void-choir-engine.js','utf8');
 // ---- determinism / structure (pure) ----
 for(const s of V.STYLE_IDS){
  const a=V.hashEvents(V.buildEventList({seed:'glacier',styleId:s,bars:8})),b=V.hashEvents(V.buildEventList({seed:'glacier',styleId:s,bars:8}));
  ok('T-01 event determinism ['+s+']',a===b,a+' vs '+b);
  const st=V.STYLES[s],c=V.createComposer('g',s),w=Object.values(st.archetypeWeights).reduce((x,y)=>x+y,0);
  ok('T-03 style envelope ['+s+'] '+st.bpm+'bpm '+c.archetype,Math.abs(w-1)<1e-9&&st.bpm>=100&&st.bpm<=200&&c.chords.length===4,'w='+w);
  const p=c.makePatterns(0.7);let n=0,mx=0;for(let g=0;g<256;g++){const t=c.eventsForStep(g,p,{}).filter(e=>e.type==='trem').length;n+=t;mx=Math.max(mx,t);}
  ok('T-21 tremolo grid, no bursts ['+s+'] n='+n+' max/step='+mx,n===Math.floor(256*st.tremPerBeat/4)&&mx<=3,n);
  ok('cowbell gated ['+s+']',c.makePatterns(1).cowbell.every(x=>!x),'fired');
  const u=new Set(c.motif).size;ok('motif has contour ['+s+'] uniq='+u,u>=5,u);
 }
 ok('T-02 seed independence',V.hashEvents(V.buildEventList({seed:'aaa',styleId:'cosmic',bars:8}))!==V.hashEvents(V.buildEventList({seed:'bbb',styleId:'cosmic',bars:8})),'collision');
 ok('T-24 style independence',new Set(V.STYLE_IDS.map(s=>V.hashEvents(V.buildEventList({seed:'x',styleId:s,bars:8})))).size===V.STYLE_IDS.length,'collision');
 ok('T-04 no Math.random in engine',!/Math\.random/.test(src),'found');
 ok('T-25 no exponential ramp targets zero (RangeError)',!/exponentialRampToValueAtTime\(\s*0\s*,/.test(src),'found');
 const cur=V.createAsymmetricDistortion(20),n=cur.length,q=Math.round(n*0.475),qm=n-1-q,asy=Math.abs(cur[q]+cur[qm]);
 ok('T-13 shaping curve is asymmetric at x=+/-0.05 ('+asy.toFixed(3)+')',asy>0.1,'symmetric');
 {const c=new OfflineAudioContext(1,128,SR),r=V.createGraph(c,'blackgaze','p');const a=r.amps.gtrtrem.distortion;
  V.createComposer('p','blackgaze').eventsForStep(0,V.emptyPattern(),{tremPerBeat:8});
  ok('T-12 tremolo rate cannot reach the lead drive node',r.amps.gtrtrem.distortion===a,'changed');
  let threw=false;try{for(let i=0;i<20;i++)r.amps.gtrtrem.setDrive(i/19);}catch(e){threw=true;}
  ok('T-13b setDrive x20 legal on a strict runtime (no InvalidStateError)',!threw,'threw');}

 // ---- guitar (baseline: original 1190-1904 Hz, inharmonic -56..-67 dB) ----
 const cs=[],is=[];
 for(const m of [40,52,64,76]){const d=await guitar(m,1.6,0.64);cs.push(D.centroid(D.spectrum(d,Math.floor(0.15*SR),4096),SR,4096,60,12000));is.push(ihr(d,fOf(m)));}
 const sp=(Math.max(...cs)-Math.min(...cs))/((Math.max(...cs)+Math.min(...cs))/2)*100;
 // T-10a retired: its 700-2600 Hz band was calibrated on the old hand-tuned filters; the reference recording's centroid is ~4.8 kHz. Superseded by T-28.
 ok('T-10b inharmonic distortion <= -45 dB at every pitch: '+is.map(x=>x.toFixed(1)).join('/'),is.every(x=>x<=-45),is);

 // ---- T-27: excitation is pitch-scale-invariant (harmonic balance at the clipper output, before the cabinet) ----
 // Windows span exactly one detune-beat cycle with a rectangular window so beat phase weights every pitch equally.
 async function excite(voice,m){const c=new OfflineAudioContext(1,Math.ceil(SR*6.6),SR),r=V.createGraph(c,'blackgaze','t');
  const amp=r.amps[voice==='lead'?'gtrtrem':'gtrchord'];amp.setDrive(0.64);amp.post.gain.value=0;amp.tap.connect(c.destination);
  V.guitarPick(c,amp,T0,{freq:fOf(m),dur:6.2,amp:voice==='lead'?0.25:0.375,isLead:voice==='lead',noise:r.noise});
  const d=(await c.startRendering()).getChannelData(0),f0=fOf(m),P=1/(f0*(Math.pow(2,10/1200)-1)),L=Math.round(P*SR),N=1<<19,mag=D.spectrumL(d,Math.floor(0.3*SR),L,N,true),E=[];
  for(let k=1;k<=10;k++){const c0=Math.round(k*f0*N/SR),w=Math.max(6,Math.round(0.03*k*f0*N/SR));let e=0;for(let i=c0-w;i<=c0+w;i++)e+=mag[i]*mag[i];E.push(10*Math.log10(e+1e-24));}
  return E.map(x=>x-E[0]);}
 for(const [voice,ms,base] of [['lead',[52,64,76,88],'10.1'],['rhythm',[36,40,48,52],'6.2']]){
  const Ls=[];for(const m of ms)Ls.push(await excite(voice,m));
  const sp=Math.max(...Array.from({length:9},(_,i)=>Math.max(...Ls.map(l=>l[i+1]))-Math.min(...Ls.map(l=>l[i+1]))));
  ok('T-27 '+voice+' excitation invariant across MIDI '+ms.join('/')+': worst spread '+sp.toFixed(1)+' dB <= 3 (original architecture: '+base+' dB)',sp<=3,sp.toFixed(1));
 }
 ok('T-27b the clipper takes the raw excitation and the measured cabinet sits after it (structure)',/tap\.connect\(cab\)/.test(src)&&/input\.connect\(ws\)/.test(src),'cabinet is not after the clipper');

 // ---- T-28: fidelity to the reference recording (regression guard; the cabinet was fitted on these notes) ----
 // Honest generalisation figure is the leave-one-pitch-out error of 5.8 dB, measured during fitting (see CHANGES.md).
 {const ref=JSON.parse(fs.readFileSync(__dirname+'/reference-profiles.json','utf8')).profiles,errs=[],per=[];
  for(const f0s of Object.keys(ref)){const f0=parseFloat(f0s),c=new OfflineAudioContext(1,SR*5,SR),r=V.createGraph(c,'raw','t');
   Object.keys(r.channels).forEach(k=>{if(k!=='gtrchord')r.channels[k].gain.gain.value=0;});r.reverbReturn.gain.value=0;r.channels.gtrchord.gain.gain.value=0.9;r.amps.gtrchord.setDrive(0.55);
   V.guitarPick(c,r.amps.gtrchord,T0,{freq:f0,dur:4.6,amp:0.375,isLead:false,noise:r.noise,refMidi:40});
   const d=(await c.startRendering()).getChannelData(0),P=1/(f0*(Math.pow(2,10/1200)-1)),L=Math.round(P*SR),N=1<<19,mag=D.spectrumL(d,Math.floor(0.4*SR),L,N,true),E=[];
   for(let k=1;k<=175;k++){let e=0;for(let i=Math.floor((k-0.5)*f0*N/SR);i<Math.floor((k+0.5)*f0*N/SR);i++)e+=mag[i]*mag[i];E.push(e);}
   const tot=E.reduce((a,b)=>a+b,0),eng=E.map(e=>Math.max(10*Math.log10(e/tot+1e-30),-90)),Km=Math.min(175,Math.floor(10000/f0)),df=eng.slice(0,Km).map((v,i)=>v-Math.max(ref[f0s][i],-65)),mu=df.reduce((a,b)=>a+b,0)/Km;
   const rms=Math.sqrt(df.reduce((a,b)=>a+(b-mu)*(b-mu),0)/Km);errs.push(rms);per.push(f0.toFixed(0)+'Hz '+rms.toFixed(1));}
  const mean=errs.reduce((a,b)=>a+b,0)/errs.length;
  ok('T-28 rhythm voice vs reference recording, 45 Hz-10 kHz: mean error '+mean.toFixed(1)+' dB <= 8.0 (hand-tuned filters: 14.9); '+per.join(', '),mean<=8.0&&Math.max(...errs)<=10.0,mean.toFixed(1));}
 // ---- T-29: the cabinet impulse response reproduces the table at any sample rate ----
 for(const sr of [44100,48000,96000]){const ir=V.makeCabIR(sr,0),N=1<<16,sp=D.spectrumL(Float64Array.from(ir),0,ir.length,N,true);let worst=0;const pts=[];
  for(const f of [150,340,500,800,1070,1700,2500,3500]){const i=Math.round(f*N/sr),g=20*Math.log10(sp[i]+1e-12),ref0=20*Math.log10(sp[Math.round(500*N/sr)]+1e-12);
   const want=V.cabDb(f)-V.cabDb(500),got=g-ref0;worst=Math.max(worst,Math.abs(got-want));pts.push(f+':'+(got-want).toFixed(1));}
  ok('T-29 cabinet IR matches the measured table at '+sr+' Hz (worst error '+worst.toFixed(1)+' dB <= 3)',worst<=3,pts.join(' '));}
 ok('T-29b cabinet IR is minimum-phase: >=95% of energy in the first 512 taps',(()=>{const ir=V.makeCabIR(44100,0);let a=0,b=0;ir.forEach((v,i)=>{b+=v*v;if(i<512)a+=v*v;});return a/b>=0.95;})(),'energy is late');

 // ---- T-30: the shipped single-file app is exactly what build.js produces from its sources ----
 {const B=require('../build.js');let built=null,err=null;try{built=B.build();}catch(e){err=e.message;}
  ok('T-30 index.html is up to date with void-choir.template.html + void-choir-engine.js (run `npm run build` if this fails)',
     built!==null&&B.norm(fs.readFileSync(B.OUT,'utf8'))===built,err||'the built page differs from its sources');}

 // ---- drums (baselines measured from original code) ----
 const kb=await drum('kick','raw',0.6);
 ok('T-16 kick reaches sub register '+D.domFreq(kb,SR,0.10,2048).toFixed(0)+' Hz',D.domFreq(kb,SR,0.10,2048)<=80,'');
 ok('T-16 kick decay < 400 ms',D.decayTo(kb,SR,-20)-T0<0.4,'');
 const sb=await drum('snare','raw',0.5),so=onset(sb);let a90=0;const spk=D.peakAbs(sb);while(Math.abs(sb[a90])<spk*0.9)a90++;
 ok('T-15 snare onset->90% '+((a90/SR-so)*1000).toFixed(2)+' ms < 5',(a90/SR-so)<0.005,'');
 const scn=D.centroid(D.spectrum(sb,Math.floor((so+0.001)*SR),2048),SR,2048,100,16000);
 ok('T-15 snare centroid '+scn.toFixed(0)+' Hz in 1.5-5 kHz',scn>1500&&scn<5000,scn);
 for(const [cy,minDec] of [['crash',0.6],['ride',0.25]]){
  const cb=await drum(cy,'blackgaze',3.4),cm=D.spectrum(cb,Math.floor((T0+0.03)*SR),8192),dec=D.decayTo(cb,SR,-20)-T0;
  const fl=D.flatness(cm,SR,8192,2000,12000),pc=D.peakCount(cm,SR,8192,2000,12000,-30);
  ok('T-14 '+cy+' dense (flat '+fl.toFixed(2)+', '+pc+' peaks; original 0.00/5) and rings '+(dec*1000).toFixed(0)+' ms',fl>=0.3&&pc>=100&&dec>=minDec&&dec<=3,'');
 }
 // ---- timing alignment: impulse through each entry point (isolates path latency from envelope shape) ----
 async function path(entry){const c=new OfflineAudioContext(1,SR*0.3|0,SR),r=V.createGraph(c,'blackgaze','t');
  Object.keys(r.channels).forEach(k=>r.channels[k].gain.gain.value=0.9);r.reverbReturn.gain.value=0;
  const b=c.createBuffer(1,64,SR);b.getChannelData(0)[0]=0.05;const sn=c.createBufferSource();sn.buffer=b;sn.connect(entry(r));sn.start(T0);
  const d=(await c.startRendering()).getChannelData(0);let bi=0,bv=0;for(let i=0;i<d.length;i++)if(Math.abs(d[i])>bv){bv=Math.abs(d[i]);bi=i;}return (bi/SR-T0)*1000;}
 const lat={drums:await path(r=>r.drumBus),bass:await path(r=>r.channels.bass.gain),rhythm:await path(r=>r.amps.gtrchord.input),lead:await path(r=>r.amps.gtrtrem.input)};
 const lv=Object.values(lat),ls=Math.max(...lv)-Math.min(...lv);
 ok('T-26 path latency spread '+ls.toFixed(1)+' ms <= 5  ('+Object.keys(lat).map(k=>k+' '+lat[k].toFixed(1)).join(', ')+')',ls<=5,'');
 // ---- mix ----
 for(const s of V.STYLE_IDS){
  const [L,R]=await full(s,'glacier',8,{intensity:1,tpb:8}),pk=Math.max(D.peakAbs(L),D.peakAbs(R));
  ok('T-11 ['+s+'] peak '+(20*Math.log10(pk+1e-9)).toFixed(2)+' dBFS <= -1',pk<=0.891,pk.toFixed(3));
  const fl=Math.floor(SR*0.05),fr=[];for(let f=SR;f+fl<L.length;f+=fl)fr.push(D.rms(L,f,f+fl));fr.sort((a,b)=>a-b);
  const pm=fr[Math.floor(fr.length*0.95)]/Math.max(fr[Math.floor(fr.length*0.5)],1e-9);
  ok('T-17 ['+s+'] level stability p95/p50 = '+pm.toFixed(2),pm<3,pm.toFixed(2));
 }
 // ---- style registry growth (gen-20260923T084400Z-ten-new-styles) ----
 ok('T-31 registry: STYLE_IDS matches STYLES keys and count is 13',
    V.STYLE_IDS.length===13&&JSON.stringify(V.STYLE_IDS)===JSON.stringify(Object.keys(V.STYLES)),'registry mismatch');
 const ARCH=['blast','gallop','halftime','sparse','punk','fourFloor','shuffle'];
 for(const s of V.STYLE_IDS){const st=V.STYLES[s];
  ok('T-32 schema ['+s+']',st.id===s&&/^[a-z0-9-]+$/.test(s)&&st.label.length>0&&
    Array.isArray(st.progression)&&st.progression.length===4&&!!V.MODES[st.mode]&&
    st.progression.every(p=>Array.isArray(p)&&p.length===2&&Number.isInteger(p[0])&&(p[1]==='maj'||p[1]==='min'))&&
    Object.keys(st.mix).slice().sort().join()===V.CHANNEL_IDS.slice().sort().join()&&
    st.drive.rhythm>=0&&st.drive.rhythm<=100&&st.drive.lead>=0&&st.drive.lead<=100&&
    st.reverb.seconds>0&&st.reverb.mix>0&&st.reverb.mix<1&&
    st.tremPerBeat>=2&&st.tremPerBeat<=8&&st.stepsPerChord>=16,'schema');
  ok('T-33 archetype keys ['+s+']',Object.keys(st.archetypeWeights).every(k=>ARCH.includes(k)),'unknown key');
 }
 ok('T-35 labels unique',new Set(Object.values(V.STYLES).map(x=>x.label)).size===V.STYLE_IDS.length,'duplicate label');
 {const fp=V.generatePatterns(V.STYLES.disco,'fourFloor',0.7,V.makeRng('t34'));
  ok('T-34 fourFloor pattern',[0,4,8,12].every(i=>fp.kick[i])&&fp.kick.filter(Boolean).length===4&&
     [1,3,5,7,9,11,13,15].every(i=>fp.ohat[i])&&fp.snare[4]&&fp.snare[12],'pattern');
  const pk=V.generatePatterns(V.STYLES.ramones,'punk',0.7,V.makeRng('t34'));
  ok('T-34b punk pattern',[0,4,8,12].every(i=>pk.kick[i])&&pk.snare[4]&&pk.snare[12]&&
     pk.chat.filter((x,i)=>i%2===0&&x).length===8&&pk.crash[0],'pattern');
  const sh=V.generatePatterns(V.STYLES['blues-rock'],'shuffle',0.7,V.makeRng('t34'));
  ok('T-34c shuffle pattern',sh.kick[0]&&sh.kick[8]&&sh.snare[4]&&sh.snare[12]&&
     sh.chat[0]&&sh.chat[4]&&sh.chat[8]&&sh.chat[12]&&sh.ride[0],'pattern');
  ok('T-36 new modes valid',V.MODES.blues.length===6&&new Set(V.MODES.blues).size===6&&
     V.MODES.mixolydian.length===7&&new Set(V.MODES.mixolydian).size===7,'modes');
 }
 console.log('\n'+pass+' passed, '+fail+' failed');process.exit(fail?1:0);
})();
