// Original 3DMK panel contract. No rendering or application-domain dependencies.
const IDS = ['workspace', 'view', 'help'];
const SIDE = {workspace:'left',view:'right',help:'right'};
const KEY = '3dmk-panels-v1';
export function createPanelState(saved) {
    const panels = Object.fromEntries(IDS.map(id => [id,{open:id==='workspace',pinned:id==='workspace'}]));
    if (saved?.version === 1) for (const id of IDS) {
        for (const key of ['open','pinned']) if (typeof saved.panels?.[id]?.[key] === 'boolean') panels[id][key]=saved.panels[id][key];
    }
    const order = IDS.filter(id=>saved?.order?.includes(id));
    for (const id of IDS) if (!order.includes(id)) order.push(id);
    return {version:1,panels,order,active:IDS.includes(saved?.active)?saved.active:'workspace',narrow:false};
}
export function visiblePanels(state) {
    const open = IDS.filter(id=>state.panels[id].open);
    if (!state.narrow) return open;
    const active = state.panels[state.active]?.open ? state.active : [...state.order].reverse().find(id=>state.panels[id].open);
    return active ? [active] : [];
}
export function reducePanels(previous, action) {
    const state = JSON.parse(JSON.stringify(previous));
    const activate = id => {state.active=id;state.order=state.order.filter(x=>x!==id).concat(id);};
    const open = id => {
        for (const other of IDS) if (other!==id && SIDE[other]===SIDE[id] && !state.panels[other].pinned) state.panels[other].open=false;
        state.panels[id].open=true;activate(id);
    };
    if (action.type==='narrow') state.narrow=!!action.narrow;
    else if (action.type==='escape') {
        const visible=new Set(visiblePanels(state));
        const id=[...state.order].reverse().find(id=>visible.has(id)&&!state.panels[id].pinned);
        if (id) state.panels[id].open=false;
    } else if (IDS.includes(action.id)) {
        const id=action.id,panel=state.panels[id];
        if (action.type==='open') open(id);
        else if (action.type==='close') panel.open=false;
        else if (action.type==='toggle') { if (visiblePanels(state).includes(id)) panel.open=false; else open(id); }
        else if (action.type==='pin') { panel.pinned=!!action.pinned; if (panel.pinned) open(id); }
        else if (action.type==='activate' && panel.open) activate(id);
    }
    return state;
}
export function mountWorkspacePanels(document, window) {
    if (window.threeDmkPanels) return window.threeDmkPanels;
    let saved; try {saved=JSON.parse(window.localStorage.getItem(KEY));} catch {}
    let state=createPanelState(saved);
    const config={workspace:{panel:'sidebar',trigger:'hamburgerBtn',pin:'workspacePinBtn',close:'workspaceCloseBtn',className:'sidebar-open'},view:{panel:'viewDrawer',trigger:'viewToggleBtn',pin:'viewPinBtn',close:'viewCloseBtn',className:'view-open'},help:{panel:'helpDrawer',trigger:'helpToggleBtn',pin:'helpPinBtn',close:'helpCloseBtn',className:'help-open'}};
    const byId=id=>document.getElementById(id);
    const render=()=>{
        const visible=new Set(visiblePanels(state));
        for(const id of IDS){
            const c=config[id],panel=byId(c.panel),show=visible.has(id),pin=byId(c.pin);
            document.body.classList.toggle(c.className,show);
            panel?.classList.toggle('open',show);panel?.toggleAttribute('inert',!show);panel?.setAttribute('aria-hidden',String(!show));
            byId(c.trigger)?.setAttribute('aria-expanded',String(show));
            pin?.setAttribute('aria-pressed',String(state.panels[id].pinned));
            pin?.setAttribute('aria-label',`${state.panels[id].pinned?'Unpin':'Pin'} ${id==='workspace'?'workspace':id} panel`);
            if(pin) pin.title=`${state.panels[id].pinned?'Unpin':'Pin'} panel`;
        }
        document.body.classList.toggle('right-panels-paired',visible.has('help')&&visible.has('view'));
        document.body.classList.toggle('panel-compact',state.narrow);
        byId('drawerScrim')?.setAttribute('aria-hidden',String(!state.narrow||!visible.size));
        try {window.localStorage.setItem(KEY,JSON.stringify({...state,narrow:false}));} catch {}
        window.dispatchEvent(new CustomEvent('3dmk-panels-changed',{detail:{visible:[...visible]}}));
    };
    const dispatch=action=>{
        const before=visiblePanels(state),focused=document.activeElement;
        state=reducePanels(state,action);const after=visiblePanels(state);render();
        const closed=before.filter(id=>!after.includes(id));
        const focusId=closed.find(id=>byId(config[id].panel)?.contains(focused)) || (action.type==='escape'?closed[0]:null);
        if(focusId) byId(config[focusId].trigger)?.focus();
    };
    for(const id of IDS){
        const c=config[id];
        byId(c.trigger)?.addEventListener('click',()=>dispatch({type:'toggle',id}));
        byId(c.close)?.addEventListener('click',()=>dispatch({type:'close',id}));
        byId(c.pin)?.addEventListener('click',()=>dispatch({type:'pin',id,pinned:!state.panels[id].pinned}));
        byId(c.panel)?.addEventListener('pointerdown',()=>dispatch({type:'activate',id}));
    }
    byId('drawerScrim')?.addEventListener('click',()=>dispatch({type:'escape'}));
    window.addEventListener('keydown',event=>{
        if(event.key!=='Escape'||document.querySelector('dialog[open]'))return;
        const before=visiblePanels(state).join();dispatch({type:'escape'});
        if(before!==visiblePanels(state).join())event.preventDefault();
    });
    const resize=()=>dispatch({type:'narrow',narrow:window.innerWidth<900});
    window.addEventListener('resize',resize);
    window.threeDmkPanels={dispatch,getState:()=>JSON.parse(JSON.stringify(state)),visible:()=>visiblePanels(state)};
    resize();return window.threeDmkPanels;
}
if(typeof document!=='undefined') mountWorkspacePanels(document,window);
