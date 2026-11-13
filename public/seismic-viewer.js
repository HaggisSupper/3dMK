// Read-only indexed slices. No amplitude volume is uploaded to the GPU.
export function mountSeismicViewer({THREE,scene,frame,displayBasis}) {
    const $=id=>document.getElementById(id);
    const panel=$('subsurfacePanel'); if(!panel)return;
    let metadata=null,slice=null,generation=0,controller=null,overlay=null;
    const status=text=>$('seismicStatus').textContent=text;
    const dispose=()=>{if(overlay){scene.remove(overlay);overlay.geometry.dispose();overlay.material.map?.dispose();overlay.material.dispose();overlay=null;}};
    const axisValues=()=>!metadata?[]:$('seismicAxis').value==='inline'?metadata.inlines:$('seismicAxis').value==='crossline'?metadata.crosslines:Array.from({length:metadata.samples},(_,i)=>i);
    const resetRange=()=>{const values=axisValues();$('seismicPosition').max=Math.max(0,values.length-1);$('seismicPosition').value='0';};
    async function request(url,options={}){const response=await fetch(url,options);const body=await response.json();if(!response.ok)throw new Error(body.error?.message||`HTTP ${response.status}`);return body;}
    function draw() {
        const canvas=$('seismicCanvas');canvas.width=slice.width;canvas.height=slice.height;
        const ctx=canvas.getContext('2d'),image=ctx.createImageData(slice.width,slice.height),scale=Math.max(Math.abs(slice.min),Math.abs(slice.max),1e-20);
        slice.amplitudes.forEach((value,i)=>{if(value==null)return;const x=Math.max(-1,Math.min(1,value/scale));image.data[i*4]=x<0?Math.round(255*(1+x)):255;image.data[i*4+1]=Math.round(255*(1-Math.abs(x)));image.data[i*4+2]=x>0?Math.round(255*(1-x)):255;image.data[i*4+3]=255;});ctx.putImageData(image,0,0);
        const time=sample=>`${(metadata.delay_ms+sample*metadata.interval_us/1000).toFixed(2)} ms`;
        $('seismicAxisLabels').textContent=slice.axis==='sample'?`Crossline ${slice.horizontal[0]}–${slice.horizontal.at(-1)} · inline ${slice.vertical[0]}–${slice.vertical.at(-1)} · sample ${slice.position} (${time(slice.position)})`:`${slice.axis==='inline'?'Crossline':'Inline'} ${slice.horizontal[0]}–${slice.horizontal.at(-1)} · sample ${slice.vertical[0]}–${slice.vertical.at(-1)} (${time(slice.vertical[0])}–${time(slice.vertical.at(-1))})`;
        status(`${metadata.traces.toLocaleString()} indexed traces · ${slice.width} × ${slice.height} preview · ${slice.samples_read.toLocaleString()} samples read · amplitude ${slice.min.toPrecision(4)} to ${slice.max.toPrecision(4)}`);
        renderOverlay();
    }
    async function refresh(){if(!metadata)return;const revision=++generation;controller?.abort();controller=new AbortController();const axis=$('seismicAxis').value,position=axisValues()[Number($('seismicPosition').value)||0];$('seismicPositionLabel').textContent=String(position);status('Reading indexed slice…');try{const result=await request(`/api/v1/seismic/${metadata.id}/slice?axis=${axis}&position=${position}&lod=${$('seismicLod').value}`,{signal:controller.signal});if(revision!==generation)return;slice=result;draw();}catch(error){if(revision===generation&&error.name!=='AbortError')status(error.message);}}
    function renderOverlay(){
        dispose();if(!slice||!$('seismicOverlay').checked)return;
        if($('seismicDomain').value!=='depth'||!$('seismicRegistered').checked){$('seismicOverlay').checked=false;status('3D well/horizon overlay requires confirmed depth sampling and coordinate registration. Time-to-depth conversion is unavailable without a velocity model.');return;}
        const value=id=>Number($(id).value),east=value('seismicEast'),north=value('seismicNorth'),top=value('seismicElevation'),dx=value('seismicCrossSpacing'),dy=value('seismicInlineSpacing'),dz=value('seismicDepthStep');
        if(![east,north,top,dx,dy,dz].every(Number.isFinite)||dx===0||dy===0||dz<=0){status('Enter finite registration values and positive depth spacing.');return;}
        const {origin=[0,0,0],renderOrigin=[0,0,0]}=frame();const w=slice.width,h=slice.height;if(w<2||h<2){status('At least two rows and columns are required for a 3D slice.');return;}
        const points=new Float32Array(w*h*3),uv=new Float32Array(w*h*2),indices=[];
        for(let r=0;r<h;r++)for(let c=0;c<w;c++){const il=slice.axis==='inline'?slice.position:slice.axis==='crossline'?slice.horizontal[c]:slice.vertical[r],xl=slice.axis==='crossline'?slice.position:slice.horizontal[c],sample=slice.axis==='sample'?slice.position:slice.vertical[r],i=r*w+c;points.set([east+(xl-metadata.crosslines[0])*dx-origin[0]-renderOrigin[0],top-sample*dz-origin[2]-renderOrigin[1],-(north+(il-metadata.inlines[0])*dy-origin[1])-renderOrigin[2]],i*3);uv.set([c/(w-1),1-r/(h-1)],i*2);if(r<h-1&&c<w-1)indices.push(i,i+1,i+w,i+1,i+w+1,i+w);}
        const geometry=new THREE.BufferGeometry();geometry.setAttribute('position',new THREE.BufferAttribute(points,3));geometry.setAttribute('uv',new THREE.BufferAttribute(uv,2));geometry.setIndex(indices);
        const texture=new THREE.CanvasTexture($('seismicCanvas'));texture.minFilter=THREE.LinearFilter;texture.magFilter=THREE.LinearFilter;texture.colorSpace=THREE.SRGBColorSpace;
        overlay=new THREE.Mesh(geometry,new THREE.MeshBasicMaterial({map:texture,side:THREE.DoubleSide,transparent:true,opacity:0.8,depthWrite:false}));overlay.name='Read-only registered seismic slice';if(displayBasis)overlay.quaternion.copy(displayBasis());scene.add(overlay);
    }
    window.addEventListener('3dmk-frame-changed',()=>{$('seismicOverlay').checked=false;$('seismicRegistered').checked=false;dispose();});
    window.addEventListener('3dmk-scene-cleared',()=>{generation++;controller?.abort();clearTimeout(sliderTimer);dispose();metadata=null;slice=null;$('seismicInput').value='';$('seismicOverlay').checked=false;$('seismicRegistered').checked=false;$('seismicCanvas').width=1;$('seismicCanvas').height=1;$('seismicPosition').value='0';$('seismicPosition').max='0';$('seismicPositionLabel').textContent='—';$('seismicAxisLabels').textContent='';status('Open a SEG-Y dataset to index traces.');});
    $('seismicInput').addEventListener('change',async event=>{const file=event.target.files[0];if(!file)return;const revision=++generation;controller?.abort();controller=new AbortController();dispose();metadata=null;slice=null;status('Streaming SEG-Y to the local Rust indexer…');try{const form=new FormData();form.append('file',file,file.name);const result=await request('/api/v1/seismic/import',{method:'POST',body:form,signal:controller.signal});if(revision!==generation)return;metadata=result;$('seismicWarnings').textContent=result.warnings.join(' ');resetRange();await refresh();}catch(error){if(revision===generation&&error.name!=='AbortError')status(error.message);}});
    $('seismicAxis').addEventListener('change',()=>{resetRange();refresh();});let sliderTimer; $('seismicPosition').addEventListener('input',()=>{clearTimeout(sliderTimer);sliderTimer=setTimeout(refresh,120);});$('seismicLod').addEventListener('change',refresh);
    $('seismicCancel').addEventListener('click',()=>{generation++;controller?.abort();status('Slice request cancelled; any late response is discarded.');});
    for(const id of ['seismicOverlay','seismicDomain','seismicRegistered','seismicEast','seismicNorth','seismicElevation','seismicCrossSpacing','seismicInlineSpacing','seismicDepthStep'])$(id).addEventListener('change',renderOverlay);
}
