import { objectsForGlb, axisBasis } from './scene-glb.js';
// Admission and lifetime guards only; Rust remains authoritative for persisted scenes.
export function validateSceneAddFiles(input, referenceWidth, hasProject) {
    const files=Array.from(input||[]);
    if(!files.length)return [];
    if(files.some(file=>!/^.+\.(glb|png|jpe?g)$/i.test(file.name)))throw new Error('Choose GLB models or PNG/JPEG references. Use Open model/package for other formats.');
    if(files.some(file=>!Number.isFinite(file.size)||file.size<=0))throw new Error('A selected file is empty or unreadable. Choose a non-empty file.');
    if(files.length>64||files.reduce((sum,file)=>sum+file.size,0)>512*1024*1024)throw new Error('Choose up to 64 files within 512 MiB total.');
    if(files.some(file=>/\.(png|jpe?g)$/i.test(file.name))&&(!Number.isFinite(Number(referenceWidth))||Number(referenceWidth)<=0))throw new Error('Enter a positive, finite reference width in metres for PNG/JPEG images.');
    const firstModel=files.findIndex(file=>/\.glb$/i.test(file.name));
    if(!hasProject&&firstModel<0)throw new Error('Add a GLB model first, or include one now, to start a scene with image references.');
    return !hasProject&&firstModel>0?[files[firstModel],...files.filter((_,index)=>index!==firstModel)]:files;
}
export function createSceneImportGate() {
    let generation=0,controller=new AbortController();
    const current=()=>{const epoch=generation,signal=controller.signal;return {signal,isCurrent:()=>epoch===generation&&!signal.aborted,assertCurrent(){if(epoch!==generation||signal.aborted)throw new Error('Scene import superseded; late result discarded.');}};};
    const invalidate=()=>{generation++;controller.abort();controller=new AbortController();};
    return {current,invalidate,begin(){invalidate();return current();}};
}
export function bindSceneClearDialog({dialog,openButton,cancelButton,confirmButton,clear}) {
    openButton.addEventListener('click',()=>{if(!dialog.open){dialog.showModal();cancelButton.focus();}});
    const cancel=()=>{if(dialog.open)dialog.close('cancel');};
    cancelButton.addEventListener('click',cancel);
    dialog.addEventListener('cancel',event=>{event.preventDefault();cancel();});
    dialog.addEventListener('close',()=>openButton.focus());
    confirmButton.addEventListener('click',()=>{if(!dialog.open)return;dialog.close('clear');clear();});
}
// 3DMK-owned scene layout. Immutable sources and editable instances remain separate.
export const identityTransform = () => ({translation:[0,0,0],rotation:[0,0,0,1],scale:[1,1,1]});
export function makeSceneObject(id, name, assetId=null, role='model') {
    return {object_id:id,name,kind:assetId?'instance':'group',role,parent_id:null,asset_id:assetId,transform:identityTransform(),visible:true,locked:false,opacity:1};
}
export async function buildSceneGroup(THREE, document, loadTemplate, clippingPlanes=[]) {
    const group=new THREE.Group(),objects=new Map(),contentRoots=new Map();
    try {
        for(const item of Object.values(document.objects)) {
            const t=item.transform;
            if(![...t.translation,...t.rotation,...t.scale,item.opacity].every(Number.isFinite)||t.scale.some(v=>v===0)||Math.abs(t.rotation.reduce((sum,v)=>sum+v*v,0)-1)>1e-6)throw new Error('Invalid scene transform');
            const visited=new Set([item.object_id]);let parent=item.parent_id;
            while(parent){const ancestor=document.objects[parent];if(!ancestor||ancestor.kind!=='group'||visited.has(parent))throw new Error('Invalid scene parent or hierarchy cycle');visited.add(parent);parent=ancestor.parent_id;}
        }
        for(const item of Object.values(document.objects)) {
            let object=new THREE.Group();
            if(item.kind==='instance') {
                const assetTemplate=await loadTemplate(item.asset_id);
                const source=item.asset_node_index===undefined||item.asset_node_index===null?assetTemplate:assetTemplate.sourceNodeTemplates?.get(item.asset_node_index);
                if(!source)throw new Error(`Missing source node ${item.asset_node_index} in asset ${item.asset_id}`);
                object.add(source.clone(true));
                let opacity=item.opacity,parent=item.parent_id;
                while(parent){const ancestor=document.objects[parent];opacity*=ancestor.opacity;parent=ancestor.parent_id;}
                object.traverse(child=>{if(child.material){const copy=m=>{const n=m.clone();n.opacity=m.opacity*opacity;n.transparent=m.transparent||n.opacity<1;n.clippingPlanes=clippingPlanes;return n;};child.material=Array.isArray(child.material)?child.material.map(copy):copy(child.material);}});
            }
            object.name=item.name;object.traverse(child=>child.userData.sceneObjectId=item.object_id);
            if(item.source_up_axis){
                if(item.kind!=='group')throw new Error('Source up-axis belongs to a source group');
                const basis=new THREE.Group();basis.quaternion.copy(axisBasis(THREE,item.source_up_axis,document.frame?.up_axis||'Y'));object.add(basis);contentRoots.set(item.object_id,basis);
            }
            object.position.fromArray(item.transform.translation);object.quaternion.fromArray(item.transform.rotation);object.scale.fromArray(item.transform.scale);object.visible=item.visible;objects.set(item.object_id,object);
        }
        for(const item of Object.values(document.objects))(item.parent_id?(contentRoots.get(item.parent_id)||objects.get(item.parent_id)):group).add(objects.get(item.object_id));
        group.updateMatrixWorld(true);
        const bounds=new THREE.Box3().setFromObject(group);
        if(!bounds.isEmpty() && [bounds.min.x,bounds.min.y,bounds.min.z,bounds.max.x,bounds.max.y,bounds.max.z].some(v=>!Number.isFinite(v)||Math.abs(v)>100000)) throw new Error('Scene placement exceeds the local rendering range. Register/rebase the asset before combining it; source coordinates remain unchanged.');
        return {group,objects};
    } catch(error) { for(const object of objects.values())object.traverse(o=>{if(o.material)for(const m of Array.isArray(o.material)?o.material:[o.material])m.dispose();});throw error; }
}
export function mountSceneWorkspace({THREE,scene,sourceGroup,canvas,camera,applyCamera,controls,loadAsset,setSourceToolsEnabled,frameChanged,onStatus=()=>{}}) {
    const $=id=>document.getElementById(id), root=new THREE.Group();root.name='Scene layout';root.visible=false;scene.add(root);
    let project=null,response=null,assets=[],selection=null,busy=false,enabled=false,renderToken=0,session=0,previewReady=false,previewError="",activeClipping=[];
    const templates=new Map(),renderObjects=new Map(),collapsed=new Set();
    const status=(text,error=false)=>{const el=$('sceneDocumentStatus');el.textContent=text;el.classList.toggle('error',error);onStatus(text,error);};
    const request=async(url,options={})=>{const r=await fetch(url,options),json=await r.json();if(!r.ok)throw new Error(json?.error?.message||`HTTP ${r.status}`);return json;};
    const endpoint=()=>`/api/v1/projects/${project}/scene`;
    const selected=()=>response?.document.objects[selection];
    const setBusy=value=>{busy=value;for(const el of $('sceneWorkspace').querySelectorAll('button,input,select'))el.disabled=value||!project;$('sceneMode').disabled=value||!project;};
    function disposeInstance(object){object.traverse(child=>{if(child.material)for(const m of Array.isArray(child.material)?child.material:[child.material])m.dispose();});}
    function disposeTemplate(value){value.traverse(o=>{o.geometry?.dispose();if(o.material)for(const m of Array.isArray(o.material)?o.material:[o.material]){for(const v of Object.values(m))if(v?.isTexture)v.dispose();m.dispose();}});}
    const ensureSession=epoch=>{if(epoch!==session)throw new Error('Scene request superseded');};
    async function template(assetId,epoch=session){
        ensureSession(epoch);
        if(templates.has(assetId))return templates.get(assetId);
        const asset=assets.find(a=>a.asset_id===assetId);if(!asset)throw new Error(`Missing asset ${assetId}`);
        if(templates.size>=64)throw new Error('Scene preview is limited to 64 unique assets');
        const used=[...templates.keys()].reduce((n,id)=>n+Number(assets.find(a=>a.asset_id===id)?.byte_length||0),0);if(used+Number(asset.byte_length||0)>512*1024*1024)throw new Error('Scene source preview budget is 512 MiB');
        const value=await loadAsset(project,asset);if(epoch!==session){disposeTemplate(value);ensureSession(epoch);}templates.set(assetId,value);return value;
    }
    async function render(){
        if(!response)return;const token=++renderToken,epoch=session;
        const {group:staged,objects}=await buildSceneGroup(THREE,response.document,id=>template(id,epoch),activeClipping);
        if(epoch!==session||token!==renderToken){disposeInstance(staged);return;}
        for(const child of [...root.children]){root.remove(child);disposeInstance(child);}
        for(const child of [...staged.children])root.add(child);renderObjects.clear();for(const [id,obj]of objects)renderObjects.set(id,obj);
        root.quaternion.copy(axisBasis(THREE,response.document.frame?.up_axis||'Y','X'));root.updateMatrixWorld(true);previewReady=true;previewError="";refreshTree();refreshInspector();
    }
    function refreshTree(){
        const tree=$('sceneObjectTree');tree.replaceChildren();if(!response)return;
        const values=Object.values(response.document.objects),children=new Map();
        for(const o of values){if(!children.has(o.parent_id))children.set(o.parent_id,[]);children.get(o.parent_id).push(o);}
        const append=(parent,depth,ancestorVisible=true)=>(children.get(parent)||[]).forEach(o=>{
            const hasChildren=children.has(o.object_id),expanded=!collapsed.has(o.object_id),effectiveVisible=ancestorVisible&&o.visible;
            const row=document.createElement('div');row.className='scene-object-row';row.style.paddingLeft=`${depth*12}px`;row.dataset.selected=String(o.object_id===selection);row.dataset.visible=String(effectiveVisible);row.setAttribute('role','treeitem');row.setAttribute('aria-level',String(depth+1));row.setAttribute('aria-selected',String(o.object_id===selection));if(hasChildren)row.setAttribute('aria-expanded',String(expanded));
            const fold=document.createElement('button');fold.className='scene-tree-fold';fold.type='button';fold.textContent=hasChildren?(expanded?'▾':'▸'):'·';fold.disabled=!hasChildren;fold.setAttribute('aria-label',`${expanded?'Collapse':'Expand'} ${o.name}`);fold.addEventListener('click',()=>{if(expanded)collapsed.add(o.object_id);else collapsed.delete(o.object_id);refreshTree();});
            const show=document.createElement('input');show.type='checkbox';show.checked=o.visible;show.title=ancestorVisible?`Show ${o.name} and its subtree`:'Hidden by an ancestor; this checkbox retains the local preference';show.setAttribute('aria-label',`Show ${o.name}`);show.disabled=busy;show.addEventListener('change',()=>command({type:'set_display',object_id:o.object_id,visible:show.checked,opacity:o.opacity}));
            const button=document.createElement('button');button.className='scene-tree-name';button.type='button';button.textContent=o.name;button.title=`${o.role} · ${o.object_id}${o.asset_node_index==null?'':` · source node ${o.asset_node_index}`}`;button.setAttribute('aria-pressed',String(o.object_id===selection));button.addEventListener('click',()=>select(o.object_id));button.addEventListener('keydown',e=>{const buttons=[...tree.querySelectorAll('.scene-tree-name')],index=buttons.indexOf(button);if(e.key==='ArrowDown'||e.key==='ArrowUp'){e.preventDefault();buttons[Math.max(0,Math.min(buttons.length-1,index+(e.key==='ArrowDown'?1:-1)))]?.focus();}else if(e.key==='ArrowRight'&&hasChildren){e.preventDefault();collapsed.delete(o.object_id);refreshTree();}else if(e.key==='ArrowLeft'&&hasChildren){e.preventDefault();collapsed.add(o.object_id);refreshTree();}});
            const lock=document.createElement('span');lock.className='scene-tree-kind';lock.textContent=o.locked?'Lock':o.kind==='group'?'':'';row.append(fold,show,button,lock);tree.append(row);if(expanded)append(o.object_id,depth+1,effectiveVisible);
        });append(null,0);
        const views=$('sceneViewSelect'),previous=views.value;views.replaceChildren(new Option('Choose view',''));
        for(const v of Object.values(response.document.views))views.add(new Option(v.name,v.view_id));views.value=response.document.views[previous]?previous:response.document.active_view_id||'';
        $('sceneUndoBtn').disabled=busy||!response.can_undo;$('sceneRedoBtn').disabled=busy||!response.can_redo;
        const fileRoots=values.filter(o=>o.parent_id===null).length;status(`Scene r${response.document.revision} · ${fileRoots} roots · ${values.length} objects`);
    }
    function select(id){selection=id;refreshTree();refreshInspector();}
    function refreshInspector(){
        const item=selected();$('sceneInspector').hidden=!item;if(!item)return;
        $('sceneSourceAxisRow').hidden=item.kind!=='group';$('sceneSourceUpAxis').value=item.source_up_axis||'';$('sceneSourceUpAxis').disabled=item.locked||busy;
        $('sceneObjectName').value=item.name;$('sceneObjectLocked').checked=item.locked;$('sceneObjectOpacity').value=item.opacity;$('sceneObjectRole').value=item.role;$('sceneRegistrationStatus').textContent=['plan','actual','top','section','slice'].includes(item.role)&&(!response.document.frame.horizontal_crs||!response.document.frame.vertical_datum)?'CRS / vertical datum unresolved. Placement is project-local.':'';
        const euler=new THREE.Euler().setFromQuaternion(new THREE.Quaternion().fromArray(item.transform.rotation),'XYZ');
        for(let i=0;i<3;i++){ $('scenePosition'+i).value=item.transform.translation[i];$('sceneRotation'+i).value=THREE.MathUtils.radToDeg(euler.toArray()[i]);$('sceneScale'+i).value=item.transform.scale[i];}
        const parent=$('sceneParent');parent.replaceChildren(new Option('Scene root',''));
        for(const object of Object.values(response.document.objects))if(object.kind==='group'&&object.object_id!==selection)parent.add(new Option(object.name,object.object_id));parent.value=item.parent_id||'';
        $('sceneApplyTransformBtn').disabled=item.locked||busy;
    }
    const captureView = name => {const snap=camera();return {view_id:crypto.randomUUID(),name,camera:snap.camera,visibility:Object.fromEntries(Object.values(response.document.objects).map(o=>[o.object_id,o.visible])),clipping:snap.clipping,section:null};};
    async function command(cmd){
        if(!project||!response||busy)return;const epoch=session,url=endpoint(),beforeCamera=JSON.stringify(response.document.camera_view);setBusy(true);
        if(cmd.type==='activate_view')cmd={...cmd,previous_view:captureView('Before view restore')};
        const body=JSON.stringify({command_id:crypto.randomUUID(),expected_revision:response.document.revision,command:cmd});
        try{
            if(['set_transform','set_source_up_axis','set_parent','add_object','add_objects'].includes(cmd.type)){
                const candidate=structuredClone(response.document);
                if(cmd.type==='set_transform')candidate.objects[cmd.object_id].transform=cmd.transform;else if(cmd.type==='set_source_up_axis')candidate.objects[cmd.object_id].source_up_axis=cmd.up_axis;else if(cmd.type==='set_parent')candidate.objects[cmd.object_id].parent_id=cmd.parent_id;else if(cmd.type==='add_objects')for(const object of cmd.objects)candidate.objects[object.object_id]=object;else candidate.objects[cmd.object.object_id]=cmd.object;
                const staged=await buildSceneGroup(THREE,candidate,id=>template(id,epoch),activeClipping);disposeInstance(staged.group);ensureSession(epoch);
            }
            let result;
            for(let attempt=0;attempt<3;attempt++){
                try{result=await request(url,{method:'POST',headers:{'Content-Type':'application/json'},body});break;}
                catch(e){ensureSession(epoch);if(!(e instanceof TypeError)||attempt===2)throw e;}
            }
            ensureSession(epoch);response=result;
            try{await render();}catch(error){ensureSession(epoch);previewReady=false;previewError=error.message;setMode(false);refreshTree();refreshInspector();status(previewError,true);}
            ensureSession(epoch);frameChanged(cmd.type==='set_source_up_axis');
            if(['activate_view','undo','redo'].includes(cmd.type)&&response.document.camera_view&&JSON.stringify(response.document.camera_view)!==beforeCamera){setMode(true);applyCamera(response.document.camera_view);}
            return response;
        }catch(error){
            if(epoch!==session)return;
            try{const recovered=await request(url);ensureSession(epoch);response=recovered;await render();}catch{}
            if(epoch===session){refreshTree();refreshInspector();status(error.message,true);}
        }finally{if(epoch===session){setBusy(false);refreshInspector();$('sceneUndoBtn').disabled=!response?.can_undo;$('sceneRedoBtn').disabled=!response?.can_redo;}}
    }
    function setMode(value){enabled=!!value&&!!project&&previewReady;root.visible=enabled;sourceGroup.visible=!enabled;$('sceneMode').value=enabled?'layout':'source';setSourceToolsEnabled(!enabled);frameChanged(true);if(value&&!previewReady)status(previewError||'Scene preview unavailable; document controls remain available',true);}
    async function attach(projectId,primaryAsset,sourceAssets){
        detach();const epoch=session;project=projectId;assets=sourceAssets;setBusy(true);
        try{
            const loaded=await request(endpoint());ensureSession(epoch);response=loaded;
            if(response.document.revision===0&&!Object.keys(response.document.objects).length){
                const base=await template(primaryAsset.asset_id,epoch);ensureSession(epoch);
                const imported=base.userData.sourceDocument?objectsForGlb(THREE,primaryAsset,base.userData.sourceDocument,makeSceneObject):{objects:[makeSceneObject(crypto.randomUUID(),primaryAsset.original_name,primaryAsset.asset_id)]};
                const candidate=structuredClone(response.document);for(const object of imported.objects)candidate.objects[object.object_id]=object;
                const staged=await buildSceneGroup(THREE,candidate,id=>template(id,epoch),activeClipping);disposeInstance(staged.group);ensureSession(epoch);
                const initialized=await request(endpoint(),{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({command_id:crypto.randomUUID(),expected_revision:0,command:{type:'add_objects',objects:imported.objects}})});
                ensureSession(epoch);response=initialized;
            }
            selection=Object.keys(response.document.objects)[0]||null;await render();ensureSession(epoch);
            setMode(true);
            const panel=$('sceneWorkspace').closest('.accordion-item');panel?.classList.add('active');panel?.querySelector('.accordion-header')?.setAttribute('aria-expanded','true');
            const view=response.document.camera_view||response.document.views[response.document.active_view_id];if(view){setMode(true);applyCamera(view);}
        }catch(e){if(epoch!==session)return;previewReady=false;previewError=e.message;setMode(false);if(!response)project=null;else{refreshTree();refreshInspector();}status(`Scene preview: ${e.message}`,true);}
        finally{if(epoch===session){setBusy(false);refreshInspector();$('sceneUndoBtn').disabled=!response?.can_undo;$('sceneRedoBtn').disabled=!response?.can_redo;}}
    }
    function detach(){session++;renderToken++;project=null;response=null;assets=[];selection=null;previewReady=false;previewError='';activeClipping=[];setMode(false);for(const child of [...root.children]){root.remove(child);disposeInstance(child);}for(const cached of templates.values())disposeTemplate(cached);templates.clear();renderObjects.clear();collapsed.clear();$('sceneObjectTree').replaceChildren();$('sceneInspector').hidden=true;$('sceneViewSelect').replaceChildren(new Option('Choose view',''));$('sceneViewName').value='';setBusy(false);status('Add a GLB or open a model/project');}
    async function addFiles(files){
        if(!project||!response||busy)return false;
        try{files=validateSceneAddFiles(files,$('sceneImageWidth').value,true);}catch(error){status(error.message,true);return false;}
        if(!files.length)return false;
        const epoch=session,projectId=project,objects=[],names=new Set(Object.values(response.document.objects).filter(o=>!o.parent_id).map(o=>o.name));
        const width=files.some(file=>/\.(png|jpe?g)$/i.test(file.name))?Number($('sceneImageWidth').value):null;
        setBusy(true);
        try{
            for(const file of files){
                status(`Adding ${file.name}…`);const form=new FormData();form.append('file',file,file.name);const asset=await request(`/api/v1/projects/${projectId}/assets`,{method:'POST',body:form});ensureSession(epoch);assets.push(asset);const base=await template(asset.asset_id,epoch);ensureSession(epoch);
                const baseName=file.name.slice(0,150);let name=baseName,count=2;while(names.has(name))name=`${baseName} (${count++})`;names.add(name);
                if(base.userData.sourceDocument){
                    const incoming=base.userData.sourceFrame?.crs;const activeAssets=new Set([...Object.values(response.document.objects),...objects].map(o=>o.asset_id).filter(Boolean));const known=new Set([...activeAssets].map(id=>templates.get(id)?.userData.sourceFrame?.crs).filter(Boolean));if(incoming&&[...known].some(crs=>crs!==incoming))throw new Error('Declared CRS values differ. Register the assets before combining them.');
                    objects.push(...objectsForGlb(THREE,asset,base.userData.sourceDocument,makeSceneObject,()=>crypto.randomUUID(),name).objects);if(objects.length+Object.keys(response.document.objects).length>4096)throw new Error('Scene limit is 4096 objects');
                }else {const o=makeSceneObject(crypto.randomUUID(),name,asset.asset_id,'reference');o.transform.scale=[width,width/(base.userData.referenceAspect||1),1];objects.push(o);}
            }
            ensureSession(epoch);setBusy(false);const result=await command({type:'add_objects',objects});ensureSession(epoch);if(!result)return false;selection=objects[0]?.object_id;setMode(true);refreshTree();refreshInspector();if(previewReady)status(`Added ${files.length} file${files.length===1?'':'s'} · source hierarchies retained · check coordinate registration`);else status(`Files saved; preview unavailable: ${previewError}`,true);return true;
        }catch(e){if(epoch===session)status(`${e.message}. Scene unchanged; uploaded sources remain in the project.`,true);return false;}finally{if(epoch===session)setBusy(false);}
    }
    $('sceneMode').addEventListener('change',e=>setMode(e.target.value==='layout'));
    $('sceneAddGroupBtn').addEventListener('click',async()=>{const o=makeSceneObject(crypto.randomUUID(),'Group');await command({type:'add_object',object:o});select(o.object_id);});
    $('sceneDuplicateBtn').addEventListener('click',async()=>{const current=selected();if(!current||current.kind!=='instance')return;const o=structuredClone(current);o.object_id=crypto.randomUUID();o.name+=' copy';o.locked=false;await command({type:'add_object',object:o});select(o.object_id);});
    $('sceneDeleteBtn').addEventListener('click',()=>selected()&&command({type:'remove_object',object_id:selection}));
    $('sceneUndoBtn').addEventListener('click',()=>command({type:'undo'}));$('sceneRedoBtn').addEventListener('click',()=>command({type:'redo'}));
    $('sceneSourceUpAxis').addEventListener('change',e=>{if(e.target.value)command({type:'set_source_up_axis',object_id:selection,up_axis:e.target.value});});
    $('sceneObjectLocked').addEventListener('change',e=>command({type:'set_locked',object_id:selection,locked:e.target.checked}));
    $('sceneObjectName').addEventListener('change',e=>command({type:'rename_object',object_id:selection,name:e.target.value}));
    $('sceneObjectRole').addEventListener('change',e=>command({type:'set_role',object_id:selection,role:e.target.value}));
    $('sceneObjectOpacity').addEventListener('change',e=>command({type:'set_display',object_id:selection,visible:selected().visible,opacity:Number(e.target.value)}));
    $('sceneParent').addEventListener('change',e=>command({type:'set_parent',object_id:selection,parent_id:e.target.value||null}));
    $('sceneApplyTransformBtn').addEventListener('click',()=>{
        const values=prefix=>[0,1,2].map(i=>$(prefix+i).value.trim()===''?NaN:Number($(prefix+i).value));const translation=values('scenePosition'),angles=values('sceneRotation'),scale=values('sceneScale');
        if(![...translation,...angles,...scale].every(Number.isFinite)){status('Transform values must be finite',true);return;}
        const q=new THREE.Quaternion().setFromEuler(new THREE.Euler(...angles.map(THREE.MathUtils.degToRad),'XYZ'));
        command({type:'set_transform',object_id:selection,transform:{translation,rotation:q.toArray(),scale}});
    });
    $('sceneSaveViewBtn').addEventListener('click',()=>{
        const name=$('sceneViewName').value.trim();if(!name){status('Name the view',true);return;}
        const view=captureView(name);
        command({type:'save_view',view});
    });
    $('sceneRestoreViewBtn').addEventListener('click',async()=>{const id=$('sceneViewSelect').value;if(!id)return;await command({type:'activate_view',view_id:id});});
    $('sceneDeleteViewBtn').addEventListener('click',()=>{const id=$('sceneViewSelect').value;if(id)command({type:'remove_view',view_id:id});});
    let down=null;canvas.addEventListener('pointerdown',e=>down=[e.clientX,e.clientY]);
    canvas.addEventListener('pointerup',e=>{
        if(!enabled||!down||Math.hypot(e.clientX-down[0],e.clientY-down[1])>4)return;
        const rect=canvas.getBoundingClientRect(),ray=new THREE.Raycaster();ray.params.Points.threshold=.5;ray.setFromCamera(new THREE.Vector2((e.clientX-rect.left)/rect.width*2-1,-(e.clientY-rect.top)/rect.height*2+1),camera().object);
        const hit=ray.intersectObject(root,true).find(h=>{for(let o=h.object;o;o=o.parent)if(!o.visible)return false;return true;});if(hit?.object.userData.sceneObjectId)select(hit.object.userData.sceneObjectId);
    });
    detach();return {attach,detach,cancelPending(){session++;renderToken++;setBusy(false);refreshTree();refreshInspector();},setMode,command,addFiles,get active(){return enabled;},get canAddFiles(){return !!project&&!!response&&previewReady&&!busy;},get document(){return response?.document;},get root(){return root;},get clipping(){return activeClipping.map(p=>({normal:p.normal.toArray(),offset_metres:p.constant}));},refreshClipping(planes){activeClipping=planes;root.traverse(o=>{if(o.material)for(const m of Array.isArray(o.material)?o.material:[o.material])m.clippingPlanes=planes;});}};
}
