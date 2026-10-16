let currentState=null;
let cubeExitTimer=null;
let raceClockStartedAt=null;
let clockAnimationFrame=0;
let lastLogoKey='';

function renderAdTicker(logos=[]){
  const key=JSON.stringify(logos);
  if(key===lastLogoKey)return;
  lastLogoKey=key;
  const footer=document.getElementById('adFooter');
  const track=document.getElementById('adTrack');
  track.style.setProperty('--ad-play-state','paused');
  track.replaceChildren();
  footer.style.display=logos.length?'flex':'none';
  if(!logos.length)return;
  for(let copy=0;copy<2;copy++){
    const group=document.createElement('div');
    group.className='ad-group';
    group.setAttribute('aria-hidden',copy?'true':'false');
    for(const logo of logos){
      const img=document.createElement('img');
      img.className='ad-logo';
      img.src=`/logos/${encodeURIComponent(logo)}`;
      img.alt=copy?'':logo.replace(/\.[^.]+$/,'');
      img.draggable=false;
      img.addEventListener('load',layoutAdTicker,{once:true});
      group.appendChild(img);
    }
    track.appendChild(group);
  }
  requestAnimationFrame(layoutAdTicker);
}

function layoutAdTicker(){
  const track=document.getElementById('adTrack');
  const first=track.firstElementChild;
  if(!first)return;
  const width=first.getBoundingClientRect().width;
  if(!width)return;
  while(track.children.length>2)track.lastElementChild.remove();
  while(track.scrollWidth<window.innerWidth+width){
    track.appendChild(first.cloneNode(true));
  }
  track.style.setProperty('--ad-offset',`-${width}px`);
  track.style.setProperty('--ad-duration',`${Math.max(12,width/65)}s`);
  track.style.setProperty('--ad-play-state','running');
}

function clockText(ms){
  const centiseconds=Math.floor(Math.max(0,ms)/10);
  return `${String(Math.floor(centiseconds/6000)).padStart(2,'0')}:${String(Math.floor(centiseconds/100)%60).padStart(2,'0')},${String(centiseconds%100).padStart(2,'0')}`;
}

function paintClock(){
  if(raceClockStartedAt===null)return;
  document.getElementById('clock').textContent=clockText(performance.now()-raceClockStartedAt);
  clockAnimationFrame=requestAnimationFrame(paintClock);
}

function syncClock(elapsedMs){
  if(elapsedMs==null){raceClockStartedAt=null;cancelAnimationFrame(clockAnimationFrame);return;}
  const now=performance.now();
  const predicted=raceClockStartedAt===null?null:now-raceClockStartedAt;
  if(predicted===null){
    raceClockStartedAt=now-elapsedMs;
    cancelAnimationFrame(clockAnimationFrame);
    clockAnimationFrame=requestAnimationFrame(paintClock);
  }else if(Math.abs(predicted-elapsedMs)>100){
    raceClockStartedAt=now-elapsedMs;
  }
}

function startCubeTransition(panel){
  document.querySelectorAll('.obs-panel.cube-out').forEach(face=>face.remove());
  const outgoing=panel.cloneNode(true);
  outgoing.removeAttribute('id');
  outgoing.querySelectorAll('[id]').forEach(node=>node.removeAttribute('id'));
  outgoing.classList.remove('animating-pro','cube-in');
  outgoing.classList.add('cube-out');
  document.getElementById('app').appendChild(outgoing);
  panel.classList.add('cube-in');
  outgoing.addEventListener('animationend',()=>outgoing.remove(),{once:true});
  panel.addEventListener('animationend',event=>{
    if(event.animationName==='next-face-in')panel.classList.remove('cube-in');
  },{once:true});
  if(cubeExitTimer)clearTimeout(cubeExitTimer);
  cubeExitTimer=setTimeout(()=>{
    outgoing.remove();
    panel.classList.remove('cube-in');
  },1250);
}

function draw(d,enabled,logos){
  renderAdTicker(logos||[]);
  if(!enabled||!d){document.getElementById('app').style.opacity='0';return;}
  document.getElementById('app').style.opacity='1';

  let newState='next';
  if(d.results_mode)newState='results';
  else if(!d.is_next&&(d.elapsed_ms!=null||(d.lanes&&d.lanes.some(x=>x.swimming||x.time!=null))))newState='live';

  const panel=document.getElementById('panel');
  const clockBug=document.getElementById('clockBug');
  if(currentState==='results'&&newState==='next')startCubeTransition(panel);
  panel.classList.remove('state-next','state-live','state-results');
  panel.classList.add(`state-${newState}`);

  if(newState==='live'){
    clockBug.classList.add('visible');
    document.getElementById('bugEvent').textContent=d.discipline_name||'PLAVECKÉ ZÁVODY';
    document.getElementById('bugSub').textContent=`${d.heat}. ROZPLAVBA`;
    syncClock(d.elapsed_ms);
  }else{
    clockBug.classList.remove('visible');
    syncClock(null);
  }

  const title=d.discipline_name||'PLAVECKÉ ZÁVODY';
  document.getElementById('panelTitle').textContent=title;
  const renderKey=JSON.stringify([newState,title,d.discipline,d.heat,d.lanes]);
  if(newState==='next')renderPrepare(d,renderKey);
  else if(newState==='results')renderResults(d,renderKey);
  else renderLive(d);
  currentState=newState;
}

async function refresh(){
  try{
    const r=await fetch('/data',{cache:'no-store'});
    const s=await r.json();
    const mode=location.pathname.startsWith('/next')?'next':'live';
    draw(s[mode],s[mode+'_enabled'],s.logos);
  }catch(_){document.getElementById('app').style.opacity='0';}
}

refresh();
setInterval(refresh,100);
