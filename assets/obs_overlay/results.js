function renderResults(d,renderKey){
  document.getElementById('panelSub').textContent=`VÝSLEDKY • ROZPLAVBA ${d.heat_index}/${d.heat_count}`;
  document.getElementById('columnTimeLabel').textContent='Výsledný čas';
  document.getElementById('columnHead').style.display='grid';
  if(renderKey!==lastRenderKey){
    document.getElementById('panelRows').innerHTML=(d.lanes||[]).filter(x=>x.rank).map((x,i)=>`
      <div class="row-item results-row" style="--row-index:${i}">
        <div class="row-lane">${x.lane}</div>
        <div class="row-club">${esc(x.club)}</div>
        <div class="row-name">${esc(x.name)}</div>
        <div class="row-entry">${x.personal_record?'<span class="result-or">OR</span>':''}${fmt(x.time)}</div>
      </div>
    `).join('');
    lastRenderKey=renderKey;
  }
}
