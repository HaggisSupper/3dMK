// DOM unit tests with explicit API fixtures. This does not qualify WebGL rendering.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
const THREE=await import(process.env.THREE_MODULE_PATH||'three');
const {parseHTML}=await import(process.env.LINKEDOM_MODULE_PATH||'linkedom');
const hierarchySource=await fs.readFile(new URL('../public/scene-glb.js',import.meta.url),'utf8');
const hierarchyUrl=`data:text/javascript;base64,${Buffer.from(hierarchySource).toString('base64')}`;
const source=(await fs.readFile(new URL('../public/scene-workspace.js',import.meta.url),'utf8')).replace("'./scene-glb.js'",JSON.stringify(hierarchyUrl));
const {mountSceneWorkspace,makeSceneObject}=await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
const html=await fs.readFile(new URL('../public/index.html',import.meta.url),'utf8');
const {document,window}=parseHTML(html);globalThis.document=document;
Object.defineProperty(window.HTMLSelectElement.prototype,'value',{get(){return this._value||'';},set(value){this._value=value;},configurable:true});
window.HTMLSelectElement.prototype.add=function(option){this.append(option);};
globalThis.Option=function(label,value){const option=document.createElement('option');option.textContent=label;option.value=value;return option;};
const asset={asset_id:'asset',original_name:'fixture.glb',byte_length:100};
const empty=(project,revision=0)=>({document:{project_id:project,revision,objects:{},views:{},active_view_id:null,frame:{up_axis:'y',horizontal_crs:null,vertical_datum:null}},can_undo:false,can_redo:false});
let posts=[],state=empty('one'),pending=null,templateTranslation=0;
globalThis.fetch=async(url,options={})=>{
  if(pending&&url.includes('/first/'))return await pending;
  if(options.method==='POST'){posts.push(JSON.parse(options.body));throw new Error('Unexpected fixture POST');}
  return {ok:true,json:async()=>structuredClone(state)};
};
const scene=new THREE.Scene(),sourceGroup=new THREE.Group(),canvas=document.createElement('canvas');
const workspace=mountSceneWorkspace({THREE,scene,sourceGroup,canvas,camera:()=>({}),applyCamera:()=>{},controls:{},setSourceToolsEnabled:()=>{},frameChanged:()=>{},loadAsset:async()=>new THREE.Mesh(new THREE.BoxGeometry(1,1,1).translate(templateTranslation,0,0),new THREE.MeshBasicMaterial())});
// A newer project must win even when an earlier request finishes later.
let resolveFirst;pending=new Promise(resolve=>resolveFirst=resolve);
const first=workspace.attach('first',asset,[asset]);state=empty('second',5);await workspace.attach('second',asset,[asset]);
resolveFirst({ok:true,json:async()=>empty('first',5)});await first;pending=null;
assert.equal(workspace.document.project_id,'second');assert.equal(posts.length,0,'revised empty scenes are not bootstrapped');
// Bootstrap must validate source bounds before committing even one object.
state=empty('large');templateTranslation=200000;await workspace.attach('large',asset,[asset]);
assert.equal(posts.length,0);assert.equal(workspace.active,false);assert.match(document.getElementById('sceneDocumentStatus').textContent,/local rendering range/);
// An invalid existing preview retains document controls and Undo availability.
const object=makeSceneObject('model','Model','asset');state=empty('existing',3);state.document.objects.model=object;state.can_undo=true;
await workspace.attach('existing',asset,[asset]);assert.equal(document.getElementById('sceneUndoBtn').disabled,false);assert.equal(workspace.active,false);
// Reparenting into a distant group fails preflight, without a server mutation.
templateTranslation=0;const parent=makeSceneObject('group','Distant group');parent.transform.translation=[200000,0,0];state.document.objects.group=parent;
await workspace.attach('existing',asset,[asset]);await workspace.command({type:'set_parent',object_id:'model',parent_id:'group'});
assert.equal(posts.length,0);assert.equal(workspace.document.objects.model.parent_id,null);assert.match(document.getElementById('sceneDocumentStatus').textContent,/local rendering range/);
// A hierarchy cycle must be rejected, not hang during parent-opacity traversal.
await workspace.command({type:'set_parent',object_id:'group',parent_id:'group'});assert.equal(posts.length,0);assert.match(document.getElementById('sceneDocumentStatus').textContent,/hierarchy cycle/);
workspace.detach();assert.equal(workspace.document,undefined);assert.equal(sourceGroup.visible,true);
console.log('PASS: project request race, empty-scene reopen, bootstrap bounds, recoverable preview, reparent bounds, cycle rejection and detach');

// Multi-file UI workflow with explicit server fixtures; no browser or WebGL mocked pass.
let assetCount=0;const apiCommands=[];state=empty('composition',5);
globalThis.fetch=async(url,options={})=>{
 if(url.endsWith('/assets')&&options.method==='POST')return {ok:true,json:async()=>({asset_id:`upload-${++assetCount}`,original_name:'same.glb',byte_length:100})};
 if(options.method==='POST'){
  const request=JSON.parse(options.body);apiCommands.push(request);assert.equal(request.expected_revision,state.document.revision);
  if(request.command.type==='add_objects')for(const object of request.command.objects){assert.ok(!state.document.objects[object.object_id]);if(object.source_up_axis)assert.ok(['x','y','z'].includes(object.source_up_axis),'Rust UpAxis wire enum');state.document.objects[object.object_id]=object;}
  else if(request.command.type==='set_display')Object.assign(state.document.objects[request.command.object_id],{visible:request.command.visible,opacity:request.command.opacity});
  else throw new Error('Unexpected UI fixture command');
  state.document.revision++;state.can_undo=true;
 }
 return {ok:true,json:async()=>structuredClone(state)};
};
const raw={scene:0,scenes:[{nodes:[0]}],nodes:[{name:'Assembly',children:[1]},{name:'Part',mesh:0,translation:[4,2,0]}]};
const layout=mountSceneWorkspace({THREE,scene:new THREE.Scene(),sourceGroup:new THREE.Group(),canvas:document.createElement('canvas'),camera:()=>({}),applyCamera:()=>{},controls:{},setSourceToolsEnabled:()=>{},frameChanged:()=>{},loadAsset:async()=>{const group=new THREE.Group();group.userData.sourceDocument=raw;group.sourceNodeTemplates=new Map([[1,new THREE.Mesh(new THREE.BoxGeometry(),new THREE.MeshBasicMaterial())]]);return group;}});
await layout.attach('composition',asset,[asset]);
assert.equal(await layout.addFiles([new File(['fixture-one'],'same.glb'),new File(['fixture-two'],'same.glb')]),true);
assert.equal(apiCommands.length,1,'one batch scene command for multi-file add');
const roots=Object.values(layout.document.objects).filter(o=>!o.parent_id);assert.equal(roots.length,2);assert.notEqual(roots[0].object_id,roots[1].object_id);assert.deepEqual(roots.map(o=>o.name),['same.glb','same.glb (2)']);
await layout.command({type:'set_display',object_id:roots[0].object_id,visible:false,opacity:1});
const rootRows=[...document.querySelectorAll('#sceneObjectTree [role=treeitem]')].filter(row=>row.getAttribute('aria-level')==='1');assert.equal(rootRows[0].dataset.visible,'false');assert.equal(rootRows[1].dataset.visible,'true');
const snapshot=structuredClone(layout.document);layout.detach();await layout.attach('composition',asset,[asset,{asset_id:'upload-1',original_name:'same.glb',byte_length:100},{asset_id:'upload-2',original_name:'same.glb',byte_length:100}]);assert.deepEqual(layout.document,snapshot);assert.equal(apiCommands.length,2,'reopen does not duplicate imported nodes');
layout.detach();console.log('PASS: multi-file UI batch, duplicate filenames, separate roots, subtree visibility and reopen without duplicate mutation');

// Appending preserves every existing source object, placement and visibility.
await layout.attach('composition',asset,[asset,{asset_id:'upload-1',original_name:'same.glb',byte_length:100},{asset_id:'upload-2',original_name:'same.glb',byte_length:100}]);
const preserved=structuredClone(layout.document.objects);
assert.equal(await layout.addFiles([new File(['third'],'another.glb')]),true);
for(const [id,object] of Object.entries(preserved))assert.deepEqual(layout.document.objects[id],object);
const countBeforeInvalid=apiCommands.length,assetsBeforeInvalid=assetCount;
assert.equal(await layout.addFiles([]),false);
assert.equal(await layout.addFiles([new File([],'empty.glb')]),false);
assert.equal(await layout.addFiles([new File(['bad'],'unsupported.obj')]),false);
document.getElementById('sceneImageWidth').value='0';
assert.equal(await layout.addFiles([new File(['image'],'image.png')]),false);
assert.equal(apiCommands.length,countBeforeInvalid);assert.equal(assetCount,assetsBeforeInvalid,'invalid inputs never upload');
// Clear during an upload must never send a scene mutation after detach.
const oldFetch=globalThis.fetch;let finishUpload;
globalThis.fetch=async(url,options={})=>url.endsWith('/assets')&&options.method==='POST'?await new Promise(resolve=>finishUpload=resolve):oldFetch(url,options);
const lateAdd=layout.addFiles([new File(['late'],'late.glb')]);
layout.detach();const detachedStatus=document.getElementById('sceneDocumentStatus').textContent;
finishUpload({ok:true,json:async()=>({asset_id:'late',original_name:'late.glb',byte_length:4})});
assert.equal(await lateAdd,false);assert.equal(apiCommands.length,countBeforeInvalid);assert.equal(layout.document,undefined);assert.equal(layout.root.children.length,0);assert.equal(document.getElementById('sceneDocumentStatus').textContent,detachedStatus);assert.equal(document.getElementById('sceneInspector').hidden,true);assert.equal(document.getElementById('sceneViewSelect').children.length,1);
globalThis.fetch=oldFetch;
console.log('PASS: additive preservation, invalid/empty/image-width admission, and detach during upload suppresses add_objects and stale UI');
