import {spawnSync} from 'node:child_process';
import {mkdirSync,writeFileSync,readFileSync,readdirSync} from 'node:fs';
import {createCipheriv,createHash} from 'node:crypto';
import path from 'node:path';
const ffmpeg=process.env.FFMPEG || "ffmpeg";
const root=path.resolve('media'); mkdirSync(root,{recursive:true});
function run(args){const r=spawnSync(ffmpeg,['-hide_banner','-loglevel','error','-y',...args],{encoding:'utf8'});if(r.status)throw Error(r.stderr);}
run(['-f','lavfi','-i','testsrc2=size=640x360:rate=30','-f','lavfi','-i','sine=frequency=880:sample_rate=48000','-t','30','-c:v','libx264','-profile:v','main','-pix_fmt','yuv420p','-preset','fast','-g','60','-keyint_min','60','-sc_threshold','0','-c:a','aac','-b:a','96k','-ac','2','-movflags','+faststart',root+'/clip.mp4']);
run(['-f','lavfi','-i','testsrc=size=640x360:rate=30','-f','lavfi','-i','sine=frequency=440:sample_rate=48000','-t','12','-c:v','libx264','-pix_fmt','yuv420p','-preset','fast','-g','60','-c:a','aac','-ac','2','-movflags','+faststart',root+'/second.mp4']);
for(const [name,type,single] of [['ts','mpegts',false],['fmp4','fmp4',false],['range','fmp4',true]]) {
 mkdirSync(root+'/'+name,{recursive:true});
 run(['-i',root+'/clip.mp4','-c','copy','-hls_time','2','-hls_list_size','0','-hls_playlist_type','vod','-hls_segment_type',type,...(single?['-hls_flags','single_file']:[]),root+'/'+name+'/index.m3u8']);
}
mkdirSync(root+'/low',{recursive:true});
run(['-i',root+'/clip.mp4','-vf','scale=320:180','-c:v','libx264','-preset','fast','-g','60','-sc_threshold','0','-b:v','180k','-c:a','aac','-hls_time','2','-hls_list_size','0','-hls_playlist_type','vod',root+'/low/index.m3u8']);
writeFileSync(root+'/master.m3u8','#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=900000,RESOLUTION=640x360\nts/index.m3u8\n#EXT-X-STREAM-INF:BANDWIDTH=300000,RESOLUTION=320x180\nlow/index.m3u8\n'.replaceAll('\\n','\n'));
const key=Buffer.from('00112233445566778899aabbccddeeff','hex');
for(const name of ['ts','fmp4']) {
 const dest=root+'/enc-'+name;mkdirSync(dest,{recursive:true});writeFileSync(dest+'/key.bin',key);
 let n=0;const lines=readFileSync(root+'/'+name+'/index.m3u8','utf8').trim().split(/\r?\n/);const out=[];
 for(const line of lines){
  if(line.startsWith('#EXT-X-MAP')){const file=/URI="([^"]+)"/.exec(line)[1];writeFileSync(dest+'/'+file,readFileSync(root+'/'+name+'/'+file));out.push(line);}
  else if(line&&!line.startsWith('#')){const iv=Buffer.alloc(16);iv.writeUInt32BE(n++,12);const cipher=createCipheriv('aes-128-cbc',key,iv);
   writeFileSync(dest+'/'+line,Buffer.concat([cipher.update(readFileSync(root+'/'+name+'/'+line)),cipher.final()]));
   out.push('#EXT-X-KEY:METHOD=AES-128,URI="key.bin",IV=0x'+iv.toString('hex'),line);
  }else out.push(line);
 }writeFileSync(dest+'/index.m3u8',out.join('\n')+'\n');
}
const files={};function walk(dir){for(const e of readdirSync(dir,{withFileTypes:true})){const f=path.join(dir,e.name);if(e.isDirectory())walk(f);else files[path.relative(root,f).replaceAll('\\','/')]=createHash('sha256').update(readFileSync(f)).digest('hex');}}walk(root);
writeFileSync('docs/media-sha256.json',JSON.stringify(files,null,2)+'\n');
writeFileSync('.runtime/media-generation.txt',spawnSync(ffmpeg,['-version'],{encoding:'utf8'}).stdout+'\n640x360 30fps H264 main yuv420p + stereo AAC 48kHz, 30 seconds; second 12 seconds.\n');
console.log('Generated '+Object.keys(files).length+' controlled media files');
