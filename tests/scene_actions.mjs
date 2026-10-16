import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
const hierarchy=await fs.readFile(new URL('../public/scene-glb.js',import.meta.url),'utf8');
const source=(await fs.readFile(new URL('../public/scene-workspace.js',import.meta.url),'utf8')).replace("'./scene-glb.js'",JSON.stringify(`data:text/javascript;base64,${Buffer.from(hierarchy).toString('base64')}`));
const {validateSceneAddFiles,createSceneImportGate,bindSceneClearDialog}=await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
assert.equal(typeof validateSceneAddFiles,'function','scene admission helper must exist');
const glb={name:'model.glb',size:10},image={name:'reference.PNG',size:10};
assert.deepEqual(validateSceneAddFiles([],null,false),[],'empty selection is a no-op');
assert.deepEqual(validateSceneAddFiles([glb],null,false),[glb]);
assert.deepEqual(validateSceneAddFiles([image,glb],2,false),[glb,image],'first model establishes a project');
assert.deepEqual(validateSceneAddFiles([image],2,true),[image]);
for(const width of ['',0,-1,NaN,Infinity,'no'])assert.throws(()=>validateSceneAddFiles([image],width,true),/width/i);
assert.throws(()=>validateSceneAddFiles([image],2,false),/GLB.*first|GLB.*start/i);
assert.throws(()=>validateSceneAddFiles([{name:'bad.obj',size:2}],null,true),/GLB.*PNG.*JPEG/);
assert.throws(()=>validateSceneAddFiles([{name:'empty.glb',size:0}],null,true),/empty/i);
assert.throws(()=>validateSceneAddFiles(Array(65).fill(glb),null,true),/64/);
assert.throws(()=>validateSceneAddFiles([{name:'large.glb',size:512*1024*1024+1}],null,true),/512/);
const gate=createSceneImportGate(),first=gate.begin();let applied=false;
let finish;const delayed=new Promise(resolve=>finish=resolve).then(()=>{first.assertCurrent();applied=true;});
gate.invalidate();assert.equal(first.signal.aborted,true);finish();await assert.rejects(delayed,/superseded/i);assert.equal(applied,false);
const second=gate.begin();assert.equal(second.isCurrent(),true);assert.equal(first.isCurrent(),false);gate.invalidate();assert.throws(()=>second.assertCurrent(),/superseded/i);
class Control extends EventTarget{focus(){this.focused=true;}click(){this.dispatchEvent(new Event('click'));}}
const dialog=new Control();dialog.open=false;dialog.showModal=function(){this.open=true;};dialog.close=function(){this.open=false;this.dispatchEvent(new Event('close'));};
const openButton=new Control(),cancelButton=new Control(),confirmButton=new Control();let clears=0;
bindSceneClearDialog({dialog,openButton,cancelButton,confirmButton,clear:()=>clears++});
openButton.click();assert.equal(dialog.open,true);assert.equal(cancelButton.focused,true);cancelButton.click();assert.equal(clears,0);assert.equal(dialog.open,false);
openButton.click();dialog.dispatchEvent(new Event('cancel',{cancelable:true}));assert.equal(clears,0);assert.equal(dialog.open,false);
openButton.click();confirmButton.click();assert.equal(clears,1);confirmButton.click();assert.equal(clears,1,'closed dialog cannot clear a second time');
console.log('PASS: scene file admission, image width, empty/invalid/budget inputs, stale import suppression, Cancel/Escape no-op and confirmed clear');

// Exercise the production loader callback bridge, including a callback after Clear.
const {default:vm}=await import('node:vm');
const html=await fs.readFile(new URL('../public/index.html',import.meta.url),'utf8');
const loaderSource=html.slice(html.indexOf('        function loadModelFile('),html.indexOf('        function processGeometries'));
let callbacks=[],processed=0,disposed=0,released=0,unloads=0;
const loaderGate=createSceneImportGate();
const context=vm.createContext({
  sceneImports:loaderGate,THREE:{DefaultLoadingManager:{}},URL:{createObjectURL:()=>`blob:test`,revokeObjectURL:()=>{}},
  Error,Promise,currentModelPackage:{},hasVertexColors:false,showLoader:()=>{},unloadCurrentModel:()=>unloads++,resetModelPackage:()=>{},
  disposeObject3D:()=>disposed++,processGeometries:()=>processed++,renderExchangeInspector:()=>{},
  gltfRenderCache:()=>({document:{},exchange:null,origin:[0,0,0],geometries:[{attributes:{position:{count:3}},userData:{},dispose(){disposed++;}}]}),
  GLTFLoader:class{setDRACOLoader(){}load(url,onLoad,onProgress,onError){callbacks.push({onLoad,onError});}},
  DRACOLoader:class{setDecoderPath(){}dispose(){}},
});
vm.runInContext(loaderSource,context);
const load=context.loadModelFile({},'glb',()=>{},undefined,()=>released++,null,loaderGate.begin());
assert.equal(processed,0,'load promise waits for the parser');
loaderGate.invalidate();await assert.rejects(load,/superseded/);
callbacks[0].onLoad({scene:{}});assert.equal(processed,0);assert.equal(disposed,1);assert.equal(released,1);assert.equal(unloads,1,'late callback cannot unload a newer workspace');
assert.deepEqual(context.currentModelPackage,{},'late GLB metadata never repopulates cleared package');
let finishAttach;const attaching=new Promise(resolve=>finishAttach=resolve);let attached=false;
const next=context.loadModelFile({},'glb',async()=>{await attaching;attached=true;},undefined,()=>released++,null,loaderGate.begin());
let finished=false;next.then(()=>finished=true);callbacks[1].onLoad({scene:{}});await Promise.resolve();
assert.equal(processed,1);assert.equal(finished,false,'source promise waits for project attachment');
finishAttach();await next;assert.equal(attached,true);assert.equal(finished,true);
console.log('PASS: production parser callback is awaitable, waits for attachment, and discards GLB metadata/geometry arriving after clear');

// Exercise production clear orchestration with explicit local-state fixture adapters.
const {parseHTML}=await import(process.env.LINKEDOM_MODULE_PATH||'linkedom');
const {document}=parseHTML(html);
const clearSource=html.slice(html.indexOf('        function clearSceneWorkspace()'),html.indexOf('        function unloadCurrentModel'));
const fixtureGate=createSceneImportGate(),pending=fixtureGate.begin();let aborted=0,detached=0,network=0;
const vector=()=>({set(){return this;}}),$=id=>document.getElementById(id);
const clearContext=vm.createContext({document,$,sceneImports:fixtureGate,processingCancelled:false,cancelProcessFlag:false,
  pendingPackageFile:{name:'pending.zip'},pendingPackageInventory:{},activeRequestController:{abort(){aborted++;}},
  THREE:{Vector3:class{}},clipPlaneX:vector(),clipPlaneY:vector(),clipPlaneZ:vector(),slicePivot:vector(),sliceSize:vector(),currentModelMaxDim:50,
  isMeasuring:true,controls:{autoRotate:true},objectAppearanceMaterials:new Set([{}]),window:{dispatchEvent(){}},Event,
  currentModelPackage:{rustProjectId:'saved-project'},loadedGeometryStore:[{}],selection:'object',measurementSets:[{}],undoStack:[{}],redoStack:[{}],
  unloadCurrentModel(){detached++;this.currentModelPackage={rustProjectId:null};this.loadedGeometryStore=[];this.selection=null;this.measurementSets=[];this.undoStack=[];this.redoStack=[];},
  setSceneImportBusy(value){this.sceneImportBusy=value;},renderExchangeInspector(){},resetProcessingButtonLabels(){},setStatus(id,message){$(id).textContent=message;},fetch(){network++;throw new Error('Clear must not contact project API');}
});
// Explicit functions avoid relying on JavaScript global method receiver binding.
clearContext.unloadCurrentModel=()=>{detached++;Object.assign(clearContext,{currentModelPackage:{rustProjectId:null},loadedGeometryStore:[],selection:null,measurementSets:[],undoStack:[],redoStack:[]});};
clearContext.setSceneImportBusy=value=>clearContext.sceneImportBusy=value;
vm.runInContext(clearSource,clearContext);clearContext.clearSceneWorkspace();
assert.equal(pending.isCurrent(),false);assert.equal(aborted,1);assert.equal(detached,1);assert.equal(network,0);
assert.equal(clearContext.currentModelPackage.rustProjectId,null);assert.equal(clearContext.pendingPackageFile,null);assert.equal(clearContext.pendingPackageInventory,null);
assert.equal(clearContext.isMeasuring,false);assert.equal(clearContext.controls.autoRotate,false);assert.equal($('enableSlicing').checked,false);assert.equal($('packageImportReview').hidden,true);
assert.equal(clearContext.sceneImportBusy,false);assert.equal(clearContext.objectAppearanceMaterials.size,0);assert.match($('sceneActionStatus').textContent,/saved projects are unchanged/);
assert.equal($('cancelClearSceneBtn').hasAttribute('autofocus'),true);assert.match($('clearSceneDescription').textContent,/Unsaved workspace edits will be lost/);
assert.equal($('sceneActions').closest('#sidebar'),null,'global actions live outside collapsible panels');
console.log('PASS: production clear resets local session/input/measurement/clipping/appearance state without project writes; confirmation and persistent-action contracts');

// Exercise the actual committed-project afterLoad callback across a delayed attach.
const committedSource=html.slice(html.indexOf('        async function loadCommittedRustPackage('),html.indexOf('        async function loadZipPackage(file)'));
const attachGate=createSceneImportGate(),attachTask=attachGate.begin();let releaseAttach,postAttachUpdates=0,loadCalls=0;
const attachWait=new Promise(resolve=>releaseAttach=resolve);
const committedContext=vm.createContext({sceneImports:attachGate,File,URL,Map,Set,Promise,Error,
  THREE:{LoadingManager:class{setURLModifier(){}}},fetch:async()=>({ok:true,blob:async()=>new Blob(['source'])}),
  currentModelPackage:{},activeGeometryRevisionId:'old',activeGeometryFingerprint:'old',rustProjectHasLocalDivergence:true,
  geometryFingerprint:()=> 'fingerprint',rememberReferenceImages:()=>{},sceneWorkspace:{attach:async()=>attachWait},
  updatePackageImagesStatus:()=>postAttachUpdates++,updateWorkflowAvailability:()=>postAttachUpdates++,
  loadModelFile:async(file,ext,afterLoad)=>{loadCalls++;await afterLoad();}
});
vm.runInContext(committedSource,committedContext);
const sourceAsset={asset_id:'asset',original_name:'model.glb',media_type:'model/gltf-binary',role:'source'};
const committed=committedContext.loadCommittedRustPackage({project:{project_id:'saved'},root_revision:{revision_id:'revision',scene_asset_id:'asset'},assets:[sourceAsset]},attachTask);
while(!loadCalls)await Promise.resolve();
attachGate.invalidate();committedContext.currentModelPackage={rustProjectId:null};releaseAttach();
await assert.rejects(committed,/superseded/);assert.equal(postAttachUpdates,0);assert.equal(committedContext.currentModelPackage.rustProjectId,null);
console.log('PASS: committed-project callback cannot restore package state or publish status after clear during attach');

// The clear adapter above is complemented by the actual source-cache teardown.
const THREE=await import(process.env.THREE_MODULE_PATH||'three');
const unloadSource=html.slice(html.indexOf('        function unloadCurrentModel('),html.indexOf('        function toggleLighting'));
const historySource=html.slice(html.indexOf('        function clearHistory()'),html.indexOf('        function captureHistoryState'));
const measuresSource=html.slice(html.indexOf('        function clearMeasurements()'),html.indexOf('        function handleMeasureClick'));
const packageSource=html.slice(html.indexOf('        const resetModelPackage ='),html.indexOf('        const rememberTextureAsset ='));
let geometryDisposals=0,textureDisposals=0;
const sourceGeometry=new THREE.BoxGeometry(),sourceGroup=new THREE.Group();sourceGeometry.addEventListener('dispose',()=>geometryDisposals++);sourceGroup.add(new THREE.Mesh(sourceGeometry,new THREE.MeshBasicMaterial()));
Object.assign(clearContext,{THREE,sceneWorkspace:{detach:()=>detached++},showLoader:()=>{},cancelCropMode:()=>{},
  loadedGeometryStore:[sourceGeometry],activeInstancedMesh:null,undoStack:[{geometries:[sourceGeometry]}],redoStack:[{}],
  disposeUnusedGeometries:geoms=>{for(const geometry of geoms||[])geometry.dispose();},updateUndoRedoUI:()=>{},
  clearSceneAnalysisOverlay:()=>{},clearSceneAnalysisContext:()=>{},currentGroup:sourceGroup,currentTexture:{dispose(){textureDisposals++;}},currentTexturePixels:{},
  filenameStem:()=> 'model',sanitizeFilename:()=> 'model',geometryRevisionSequence:0,resetFloatingMeshReviewState:()=>{},
  clearBlueprintPlane:()=>{},floorplanLayoutGroup:{},measurementSets:[{}],activeMeasurePoints:[1],activeMeasurementEdit:{},measurementIdSequence:3,measurementCounters:{distance:1,angle:1,area:1},
  removeMeasureLabel:()=>{},clearGroupChildren:group=>group.clear(),measurementGroup:new THREE.Group(),measurementDraftGroup:new THREE.Group(),
  hasVertexColors:true,sceneManualRotation:new THREE.Euler(1,2,3),axisBasis:()=>new THREE.Quaternion(),controls:{target:new THREE.Vector3(),update(){}},activeCamera:new THREE.PerspectiveCamera(),
  buildSceneGraph:()=>{},updateCanvasEmptyState:()=>{},updatePackageImagesStatus:()=>{},updateWorkflowAvailability:()=>{},setMeasureHint:()=>{},updateContextHelp:()=>{}
});
vm.runInContext(packageSource+historySource+measuresSource+unloadSource,clearContext);
clearContext.unloadCurrentModel();
assert.equal(clearContext.loadedGeometryStore.length,0);assert.equal(clearContext.currentGroup.children.length,0);assert.equal(clearContext.currentTexture,null);assert.equal(clearContext.currentTexturePixels,null);assert.equal(textureDisposals,1);assert.ok(geometryDisposals>0);
assert.equal(clearContext.undoStack.length,0);assert.equal(clearContext.redoStack.length,0);assert.equal(clearContext.measurementSets.length,0);assert.equal(clearContext.activeMeasurePoints.length,0);assert.equal(clearContext.activeMeasurementEdit,null);assert.equal(clearContext.currentModelPackage.rustProjectId,null);assert.equal(clearContext.hasVertexColors,false);
console.log('PASS: actual source teardown empties geometry, project package, measurements and local undo/redo and disposes texture/cache resources');

// The independently indexed seismic preview must observe the same global clear.
const seismicSource=(await fs.readFile(new URL('../public/seismic-viewer.js',import.meta.url),'utf8')).replace('export function mountSeismicViewer','function mountSeismicViewer');
const seismicDocument=parseHTML(html).document,seismicWindow=new EventTarget();let importSeismic,resolveSeismic,seismicFetches=0,seismicSignal;
const seismicInput=seismicDocument.getElementById('seismicInput');seismicInput.addEventListener=(type,listener)=>{if(type==='change')importSeismic=listener;};
const seismicContext=vm.createContext({document:seismicDocument,window:seismicWindow,Event,AbortController,FormData,setTimeout,clearTimeout,
  fetch:async(url,options)=>{seismicFetches++;seismicSignal=options.signal;return await new Promise(resolve=>resolveSeismic=resolve);}
});
vm.runInContext(seismicSource,seismicContext);seismicContext.mountSeismicViewer({THREE,scene:new THREE.Scene(),frame:()=>({})});
const indexing=importSeismic({target:{files:[new File(['source'],'scan.sgy')]}});
seismicWindow.dispatchEvent(new Event('3dmk-scene-cleared'));assert.equal(seismicSignal.aborted,true);
resolveSeismic({ok:true,json:async()=>({id:'late',inlines:[1],crosslines:[2],samples:1,warnings:[]})});await indexing;
assert.equal(seismicFetches,1,'late indexing response cannot request a slice after clear');assert.equal(seismicDocument.getElementById('seismicStatus').textContent,'Open a SEG-Y dataset to index traces.');assert.equal(seismicDocument.getElementById('seismicCanvas').width,1);
console.log('PASS: global clear invalidates indexed seismic import and suppresses its late slice/status publication');

// Regression: cancellation from Clear must not poison the next ZIP workflow.
const zipDocument=parseHTML(html).document,zipGate=createSceneImportGate();
Object.defineProperty(zipDocument.getElementById('packageModelSelect'),'value',{get(){return this._value||'';},set(value){this._value=value;},configurable:true});
const functionSlice=(start,end)=>html.slice(html.indexOf(start),html.indexOf(end));
let zipInspections=0,zipCommits=0,zipAttachments=0;
const inventory={primary_model:{selected_path:'model.glb',candidates:[{path:'model.glb',format:'glb',score:1}]},entries:[],file_count:1,total_expanded_bytes:10};
const zipContext=vm.createContext({document:zipDocument,$:id=>zipDocument.getElementById(id),sceneImports:zipGate,sceneImportBusy:false,
  File,FormData,AbortController,Error,Promise,Event,setTimeout,clearTimeout,
  THREE:{Vector3:class{}},clipPlaneX:vector(),clipPlaneY:vector(),clipPlaneZ:vector(),slicePivot:vector(),sliceSize:vector(),currentModelMaxDim:10,
  controls:{autoRotate:false},objectAppearanceMaterials:new Set(),window:{dispatchEvent(){}},isMeasuring:false,
  currentModelPackage:{rustProjectId:'old-project'},pendingPackageFile:{},pendingPackageInventory:{},activeRequestController:null,activeServerJobId:null,processingCancelled:false,cancelProcessFlag:false,
  unloadCurrentModel:()=>zipContext.currentModelPackage={rustProjectId:null},renderExchangeInspector:()=>{},resetProcessingButtonLabels:()=>{},
  setSceneImportBusy:value=>zipContext.sceneImportBusy=value,
  setStatus:(id,message)=>zipDocument.getElementById(id).textContent=message,
  showLoader:visible=>{if(visible)zipContext.processingCancelled=false;},
  getProjectUnitKey:()=> 'm',filenameStem:()=> 'package',escapeHtml:value=>String(value),
  loadCommittedRustPackage:async(imported,task)=>{task.assertCurrent();zipAttachments++;zipContext.currentModelPackage.rustProjectId=imported.project.project_id;},
  fetch:async url=>{
    let result;
    if(url==='/api/v1/packages/inspect'){zipInspections++;result={inventory};}
    else if(url==='/api/v1/projects/import-package'){zipCommits++;result={project:{project_id:'new-project'},assets:[sourceAsset],root_revision:{revision_id:'revision-new'}};}
    else throw new Error(`Unexpected ZIP regression request ${url}`);
    return {ok:true,text:async()=>JSON.stringify(result)};
  }
});
vm.runInContext(clearSource+
  functionSlice('        async function fetchJson(', '        function setStatus(')+
  functionSlice('        async function handleModelUpload(', '        function renderExchangeInspector(')+
  functionSlice('        async function inspectZipPackage(', '        async function loadCommittedRustPackage('),zipContext);
zipContext.clearSceneWorkspace();assert.equal(zipContext.processingCancelled,true);
await zipContext.handleModelUpload({target:{files:[new File(['zip fixture'],'project.zip')],value:'project.zip'}});
assert.equal(zipInspections,1);assert.ok(zipContext.pendingPackageInventory,'Clear followed by ZIP inspection must publish the new inventory');
assert.equal(zipDocument.getElementById('importPackageBtn').disabled,false);
await zipContext.commitPendingPackage();
assert.equal(zipCommits,1);assert.equal(zipAttachments,1);assert.equal(zipContext.currentModelPackage.rustProjectId,'new-project');assert.equal(zipContext.pendingPackageInventory,null);assert.equal(zipContext.sceneImportBusy,false);
console.log('PASS: actual Clear → ZIP selection → inspection → commit accepts fresh requests and attaches the new project');

// Secondary decoders and Stop share the same captured workspace lifetime.
const processingDocument=parseHTML(html).document,processingGate=createSceneImportGate();
Object.defineProperty(processingDocument.getElementById('pointCloudFile'),'files',{value:[new File(['source'],'cloud.ply')],configurable:true});
let plyCallbacks=[],processedSecondary=0,disposedSecondary=0,secondaryStatuses=[],textureLoads=0,resolveRefined;
const processingContext=vm.createContext({...clearContext,document:processingDocument,$:id=>processingDocument.getElementById(id),sceneImports:processingGate,
  THREE,File,FormData,Promise,Error,URL:{createObjectURL:()=> 'blob:decoder',revokeObjectURL(){}},
  backendCapabilities:{meshReconstruction:true},sceneWorkspace:{cancelPending(){}},activeRequestController:null,activeServerJobId:null,
  PLYLoader:class{load(url,onLoad){plyCallbacks.push(onLoad);}},
  fetchJson:async()=>({mesh_file:'/mesh.ply',refined_texture:'/texture.png'}),
  processGeometries:()=>processedSecondary++,showLoader:()=>{},resetModelPackage:()=>{},resetProcessingButtonLabels:()=>{},
  unloadCurrentModel:()=>{processingContext.loadedGeometryStore=[];processingContext.currentModelPackage={rustProjectId:null};},
  setSceneImportBusy:value=>processingContext.sceneImportBusy=value,
  setStatus:(id,message)=>{secondaryStatuses.push([id,message]);processingDocument.getElementById(id).textContent=message;},
  renderExchangeInspector:()=>{},loadTextureFile:async()=>textureLoads++,updateWorkflowAvailability:()=>{},
  fetch:async()=>await new Promise(resolve=>resolveRefined=resolve)
});
vm.runInContext(clearSource+loaderSource+
  functionSlice('        function loadPlyGeometryFromUrl(', '        function buildRustTransformMatrix(')+
  functionSlice('        async function runPoissonBackend(', '        function getActiveTexture(')+
  functionSlice('        async function cancelActiveProcessing(', '        function showLoader(')+
  functionSlice('        async function refinePackageTexture(', '        function projectionOptionsFromUi('),processingContext);
const poisson=processingContext.runPoissonBackend();while(!plyCallbacks.length)await Promise.resolve();
processingContext.clearSceneWorkspace();const statusesAfterClear=secondaryStatuses.length;
plyCallbacks[0]({index:null,attributes:{},userData:{},dispose(){disposedSecondary++;}});await poisson;
assert.equal(processedSecondary,0);assert.equal(disposedSecondary,1);assert.equal(secondaryStatuses.length,statusesAfterClear,'late secondary decode cannot overwrite reset status');
const pendingParser=processingContext.loadModelFile({},'ply',()=>{},undefined,()=>{},null,processingGate.begin());
processingContext.sceneImportBusy=true;
await processingContext.cancelActiveProcessing();await assert.rejects(pendingParser,/superseded/);
plyCallbacks[1]({index:null,attributes:{position:{count:3}},userData:{},dispose(){disposedSecondary++;}});
assert.equal(processedSecondary,0);assert.equal(processingContext.sceneImportBusy,false);assert.equal(disposedSecondary,2);
processingContext.currentModelPackage={textureBlob:new Blob(['texture']),textureName:'texture.png',referenceImages:[{blob:new Blob(['photo']),name:'photo.png'}]};
const refinement=processingContext.refinePackageTexture();while(!resolveRefined)await Promise.resolve();
processingContext.clearSceneWorkspace();const statusesAfterRefineClear=secondaryStatuses.length;
resolveRefined({ok:true,blob:async()=>new Blob(['refined'])});await refinement;
assert.equal(textureLoads,0);assert.equal(secondaryStatuses.length,statusesAfterRefineClear,'late texture download cannot begin a fresh image task or publish status');
console.log('PASS: Clear during backend PLY decode disposes output, Stop rejects primary parser and releases busy state, and late refined download never re-enters texture loading');

// Local operations must not consume captured pre-clear geometry after yielding.
const localYieldCallbacks=[];processingContext.setTimeout=callback=>{localYieldCallbacks.push(callback);return 0;};
processingContext.getLoadedGeometryMode=()=> 'mesh';processingContext.loadedGeometryStore=[{userData:{isMesh:true},attributes:{position:{count:3}}}];
vm.runInContext(functionSlice('        async function convertMeshToPointCloud(', '        function classifyGeometry(')+functionSlice('        async function applyDestructiveCrop(', '        // --- Backend API: Floorplan'),processingContext);
const sampling=processingContext.convertMeshToPointCloud();assert.equal(localYieldCallbacks.length,1);processingContext.clearSceneWorkspace();const sampleStatuses=secondaryStatuses.length;localYieldCallbacks.shift()();await sampling;
assert.equal(processedSecondary,0);assert.equal(secondaryStatuses.length,sampleStatuses);
processingContext.cropBoxMesh={userData:{currentBounds:{}}};const cropping=processingContext.applyDestructiveCrop();assert.equal(localYieldCallbacks.length,1);processingContext.clearSceneWorkspace();processingContext.cropBoxMesh=null;localYieldCallbacks.shift()();await cropping;
assert.equal(processedSecondary,0);
console.log('PASS: Clear during local crop/sampling yield prevents cached source geometry from being republished');

// Capability reads belong to the app, so scene changes must not poison startup.
const healthDocument=parseHTML(html).document,healthGate=createSceneImportGate();let releaseHealth,releaseVlm;
const healthContext=vm.createContext({document:healthDocument,$:id=>healthDocument.getElementById(id),sceneImports:healthGate,
  AbortController,setTimeout,clearTimeout,Error,activeRequestController:null,activeServerJobId:null,currentModelPackage:{rustProjectId:null},backendCapabilities:{},
  updateWorkflowAvailability(){},updateContextHelp(){},setStatus(){},
  fetch:async url=>{
    if(url==='/api/health')return await new Promise(resolve=>releaseHealth=resolve);
    if(url==='/api/v1/vlm/status')return await new Promise(resolve=>releaseVlm=resolve);
    throw new Error(`Unexpected capability request ${url}`);
  }
});
vm.runInContext(functionSlice('        async function fetchJson(', '        function setStatus(')+functionSlice('        async function checkBackend(', '        function getLoadedGeometryMode(')+functionSlice('        async function checkVlmRuntime(', '        function vlmDetectionOptionsFromUi('),healthContext);
const healthCheck=healthContext.checkBackend();
assert.equal(healthContext.activeRequestController,null,'health must not replace the active scene-operation controller');
healthGate.begin();releaseHealth({ok:true,text:async()=>JSON.stringify({capabilities:{point_cloud_to_mesh:true}})});
while(!releaseVlm)await Promise.resolve();
assert.equal(healthContext.activeRequestController,null,'VLM probe is not a scene operation');
healthGate.invalidate();releaseVlm({ok:true,text:async()=>JSON.stringify({reachable:true,model:'local-model'})});await healthCheck;
assert.equal(healthContext.backendCapabilities.connected,true);assert.equal(healthContext.backendCapabilities.meshReconstruction,true);assert.equal(healthContext.backendCapabilities.vlm,true);assert.equal(healthDocument.getElementById('backendStatus').className,'backend-status connected');
console.log('PASS: delayed health and VLM capability probes survive scene-generation changes without turning the backend offline');
