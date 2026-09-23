function fft(re,im){const n=re.length;let j=0;for(let i=1;i<n;i++){let bit=n>>1;for(;j&bit;bit>>=1)j^=bit;j^=bit;if(i<j){[re[i],re[j]]=[re[j],re[i]];[im[i],im[j]]=[im[j],im[i]];}}
for(let m=2;m<=n;m<<=1){const ang=-2*Math.PI/m,wr=Math.cos(ang),wi=Math.sin(ang);for(let i=0;i<n;i+=m){let cr=1,ci=0;for(let k=0;k<m/2;k++){const ur=re[i+k],ui=im[i+k];const vr=re[i+k+m/2]*cr-im[i+k+m/2]*ci;const vi=re[i+k+m/2]*ci+im[i+k+m/2]*cr;re[i+k]=ur+vr;im[i+k]=ui+vi;re[i+k+m/2]=ur-vr;im[i+k+m/2]=ui-vi;const ncr=cr*wr-ci*wi;ci=cr*wi+ci*wr;cr=ncr;}}}}
function spectrum(d,off,N){const re=new Float64Array(N),im=new Float64Array(N);for(let i=0;i<N;i++){const s=d[off+i]||0;re[i]=s*(0.5-0.5*Math.cos(2*Math.PI*i/(N-1)));}fft(re,im);const m=new Float64Array(N/2);for(let i=0;i<N/2;i++)m[i]=Math.hypot(re[i],im[i]);return m;}
function centroid(mag,sr,N,lo,hi){const a=Math.floor(lo*N/sr),b=Math.min(mag.length,Math.ceil(hi*N/sr));let num=0,den=0;for(let i=a;i<b;i++){num+=i*sr/N*mag[i];den+=mag[i];}return den>0?num/den:0;}
function flatness(mag,sr,N,lo,hi){const a=Math.max(1,Math.floor(lo*N/sr)),b=Math.min(mag.length,Math.ceil(hi*N/sr));let lg=0,s=0,n=0;for(let i=a;i<b;i++){const m=mag[i]+1e-12;lg+=Math.log(m);s+=m;n++;}return n?Math.exp(lg/n)/(s/n):0;}
function peakCount(mag,sr,N,lo,hi,fdb){const a=Math.max(1,Math.floor(lo*N/sr)),b=Math.min(mag.length-1,Math.ceil(hi*N/sr));let mx=0;for(let i=a;i<b;i++)mx=Math.max(mx,mag[i]);const thr=mx*Math.pow(10,fdb/20);let c=0;for(let i=a+1;i<b-1;i++)if(mag[i]>thr&&mag[i]>mag[i-1]&&mag[i]>=mag[i+1])c++;return c;}
function peakAbs(d){let m=0;for(let i=0;i<d.length;i++)m=Math.max(m,Math.abs(d[i]));return m;}
function rms(d,a,b){let s=0,n=0;for(let i=a;i<b&&i<d.length;i++){s+=d[i]*d[i];n++;}return Math.sqrt(s/Math.max(1,n));}
function attackTime(d,sr){const pk=peakAbs(d);for(let i=0;i<d.length;i++)if(Math.abs(d[i])>=pk*0.9)return i/sr;return Infinity;}
function decayTo(d,sr,db){const pk=peakAbs(d),thr=pk*Math.pow(10,db/20);for(let i=d.length-1;i>=0;i--)if(Math.abs(d[i])>thr)return i/sr;return 0;}
function domFreq(d,sr,t,N){const off=Math.floor(t*sr),mag=spectrum(d,off,N);let bi=1,bv=0;for(let i=1;i<mag.length;i++)if(mag[i]>bv){bv=mag[i];bi=i;}return bi*sr/N;}
function spectrumL(d,off,L,N,rect){const re=new Float64Array(N),im=new Float64Array(N);for(let i=0;i<L;i++){const w=rect?1:0.5-0.5*Math.cos(2*Math.PI*i/(L-1));re[i]=(d[off+i]||0)*w;}fft(re,im);const m=new Float64Array(N/2);for(let i=0;i<N/2;i++)m[i]=Math.hypot(re[i],im[i]);return m;}
module.exports={spectrumL,spectrum,centroid,flatness,peakCount,peakAbs,rms,attackTime,decayTo,domFreq};
