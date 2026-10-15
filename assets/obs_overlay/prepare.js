const esc=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const fmt=ms=>{if(ms==null)return'00:00,00';let cs=Math.floor(ms/10),m=Math.floor(cs/6000),s=Math.floor(cs%6000/100),h=cs%100;return `${String(m).padStart(2,'0')}:${String(s).padStart(2,'0')},${String(h).padStart(2,'0')}`};

let lastRenderKey='';

function renderPrepare(d,renderKey){
  document.getElementById('panelSub').textContent=`STARTOVNÍ LISTINA • ROZPLAVBA ${d.heat_index}/${d.heat_count}`;
  document.getElementById('columnTimeLabel').textContent='Přihlášený čas';
  document.getElementById('columnHead').style.display='grid';
  if(renderKey!==lastRenderKey){
    document.getElementById('panelRows').innerHTML=(d.lanes||[]).map((x,i)=>`
      <div class="row-item" style="--row-index:${i}">
        <div class="row-lane">${x.lane}</div>
        <div class="row-club">${esc(x.club)}</div>
        <div class="row-name">${esc(x.name)}</div>
        <div class="row-entry">${esc(x.entry)}</div>
      </div>
    `).join('');
    lastRenderKey=renderKey;
  }
}
