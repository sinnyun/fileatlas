// Regression for the user-reported dark-on-dark thumbnail labels. Isolated same-process MCP.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const zlib = require('node:zlib');

module.exports = async ({step, delay, root, profile}) => {
  const invoke = async (tool, args = {}, extra = {}) => (await step({tool, arguments: args, ...extra})).result;
  const state = () => invoke('get_ui_state');
  const ui = (action, value, extra = {}) => invoke('control_ui', {action, ...(value === undefined ? {} : {value})}, {wait_ms:200,...extra});
  const gallery = (action, args = {}) => invoke('control_gallery', {action,...args}, {wait_ms:200});
  const until = async check => {
    for (let i=0;i<100;i++) { const current=await state(); if(check(current)) return current; await delay(150); }
    throw Error('thumbnail contrast UI did not settle');
  };
  const click = async (x,y) => {
    for(const action of ['pointer_move','pointer_down','pointer_up']) await ui(action,`${x},${y}`);
  };
  // Create valid black/white/colored PNGs inside the isolated profile, never modify source photos.
  const crc = data => { let c=0xffffffff; for(const byte of data) { c^=byte; for(let k=0;k<8;k++) c=(c>>>1)^((c&1)?0xedb88320:0); } return (c^0xffffffff)>>>0; };
  const chunk = (name,data) => { const type=Buffer.from(name),size=Buffer.alloc(4),sum=Buffer.alloc(4);size.writeUInt32BE(data.length);sum.writeUInt32BE(crc(Buffer.concat([type,data])));return Buffer.concat([size,type,data,sum]); };
  const png = color => {
    const width=160,height=160,header=Buffer.alloc(13);header.writeUInt32BE(width);header.writeUInt32BE(height,4);header[8]=8;header[9]=2;
    const rows=Buffer.alloc((width*3+1)*height);
    for(let y=0;y<height;y++) for(let x=0;x<width;x++) color.forEach((v,c)=>rows[y*(width*3+1)+1+x*3+c]=v);
    return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),chunk('IDAT',zlib.deflateSync(rows)),chunk('IEND',Buffer.alloc(0))]);
  };
  const fixture=path.join(profile,'contrast-files');fs.mkdirSync(fixture,{recursive:true});
  for(const [name,color] of [['01-black.png',[0,0,0]],['02-white.png',[255,255,255]],['03-blue.png',[65,104,134]]]) fs.writeFileSync(path.join(fixture,name),png(color));
  fs.copyFileSync(path.join(root,'test-data/photag-library/01-small/small.png'),path.join(fixture,'04-photo.png'));
  for(const extension of ['psd','ipk','zip','toml','py','svg','txt','']) {
    const filename='format'+(extension?'.'+extension:'');
    fs.writeFileSync(path.join(fixture,filename),extension==='svg'?'<svg xmlns="http://www.w3.org/2000/svg" width="160" height="160"><rect width="160" height="160" fill="white"/></svg>':'Contrast regression fixture');
  }
  await delay(1500);
  await invoke('material-theme.generate', {seed:'#6686a3',variant:'neutral',contrast:0.5,apply:true});
  await delay(400);
  await gallery('navigate',{value:fixture});
  await until(s=>s.gallery?.matched_count===12 && !s.browser.status[2]);
  const files=(await invoke('gallery.search_media',{limit:50})).items;
  const images=files.filter(f=>/^0[1-4]-/.test(f.filename));assert.equal(images.length,4);
  for(const [name,color] of [['紫','#763c98'],['蓝','#315ca8'],['白','#ffffff'],['黑','#000000']]) {
    const created=await invoke('navigation.create_tag',{name});const tag=created.tag;
    assert(tag?.id,'create_tag must return the production tag');
    await invoke('control_navigation',{action:'tag_appearance',tag_id:tag.id,color,icon:'Tag'});
    await invoke('tagging.assign',{media_ids:images.map(f=>f.id),tag_ids:[tag.id]});
  }
  await invoke('gallery.set_rating',{media_ids:images.map(f=>f.id),rating:4});
  for(const image of images) {
    const detail=await invoke('gallery.get_media',{media_id:image.id});
    assert.equal(detail.media.rating,4);assert.equal(detail.tags.filter(t=>t.display).length,4);
  }
  await gallery('refresh');await until(s=>!s.browser.status[2]);
  await ui('deselect');await delay(1200);
  const luminance = color => {
    const values=[16,8,0].map(shift=>{const v=((color>>shift)&255)/255;return v<=0.04045?v/12.92:((v+0.055)/1.055)**2.4;});
    return values[0]*0.2126+values[1]*0.7152+values[2]*0.0722;
  };
  const ratios=[];
  const check = (mode, label,foreground,background) => {
    const a=luminance(foreground),b=luminance(background),ratio=(Math.max(a,b)+0.05)/(Math.min(a,b)+0.05);
    ratios.push({mode,label,ratio});assert(ratio>=4.5,`${mode} ${label}: ${ratio.toFixed(2)}:1`);
  };
  const verify = async mode => {
    const theme=(await state()).theme;assert.equal(theme.mode,mode);const colors=theme.content_colors;
    for(const format of colors.formats) check(mode,`format ${format.extension||'FILE'}`,format.foreground,format.background);
    for(const tag of colors.tags) {
      check(mode,`tag ${tag.tint}/${tag.selected} text`,tag.foreground,tag.background);
      check(mode,`tag ${tag.tint}/${tag.selected} icon`,tag.icon,tag.background);
    }
    check(mode,'rating',colors.media_star,colors.media_background);
    check(mode,'play/overflow',colors.media_text,colors.media_background);
    check(mode,'navigation selected',colors.selection_text,theme.selection);
  };
  // The footer button's location is read from the same-process rendered geometry.
  let s=await state();
  fs.writeFileSync(path.join(profile,'contrast-start-state.json'),JSON.stringify(s,null,2));
  // Current host footer theme center is 116 logical px above the client bottom.
  const geometry=s.window_status.geometry;
  const clientHeight=geometry.client_size.height;
  const toggleMode=async()=>{assert(Number.isFinite(clientHeight),JSON.stringify(geometry));await click(22,clientHeight-116);await delay(250);};
  if(s.theme.mode!=='light') await toggleMode();
  await verify('light');
  await invoke('get_ui_state',{}, {capture:'thumbnail-light'});
  await ui('hover_media',String(images[0].id));await delay(300);
  await invoke('get_ui_state',{}, {capture:'thumbnail-light-hover'});await ui('hover_clear');
  await gallery('popover',{value:'extensions'});await delay(300);
  await invoke('get_ui_state',{}, {capture:'extensions-light'});await gallery('popover',{value:'close'});
  await toggleMode();await verify('dark');await invoke('get_ui_state',{}, {capture:'thumbnail-dark'});
  const quiet=await state();await delay(1000);assert.equal((await state()).gallery.snapshot_revision,quiet.gallery.snapshot_revision,'settled colors must not reload Gallery');
  await toggleMode();await verify('light');await ui('settings_open');await ui('settings_tab','tags');await delay(400);
  await invoke('get_ui_state',{}, {capture:'tag-settings-light'});await ui('settings_close');
  fs.writeFileSync(path.join(profile,'contrast-summary.json'),JSON.stringify({passed:true,ratios,fixture,geometry,remaining:['native playback badge not separately exercised','small-window/keyboard focus/disabled states outside this screenshot fix']},null,2));
};
