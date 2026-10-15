function renderLive(d){
  document.getElementById('panelPhase').textContent='LIVE';
  document.querySelector('.bug-sub').textContent=`ROZPLAVBA ${d.heat_index}/${d.heat_count}`;
  document.getElementById('columnHead').style.display='none';

  // Překresli strukturu jen při změně plavců; časy měň bez blikání.
  const list=d.lanes||[];
  const liveStructureKey=JSON.stringify(list.map(x=>[x.lane,x.name,x.club]));
  if(liveStructureKey!==lastRenderKey){
    document.getElementById('panelRows').innerHTML=list.map(x=>`
      <div class="live-row-item">
        <div class="live-lane">${x.lane}</div>
        <div class="live-identity">
          <div class="live-swimmer">${esc(x.name)}</div>
          <div class="live-club-text">${esc(x.club)}</div>
        </div>
        <div class="live-value" data-live-value="${x.lane}"></div>
      </div>
    `).join('');
    lastRenderKey=liveStructureKey;
  }
  for(const x of list){
    const valEl=document.querySelector(`[data-live-value="${x.lane}"]`);
    if(valEl){
      const next=x.time!=null?fmt(x.time):'';
      if(valEl.textContent!==next)valEl.textContent=next;
    }
  }
}
