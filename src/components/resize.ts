export const resizeValue=(value:number,delta:number,min:number,max:number)=>Math.min(max,Math.max(min,value+delta));
export function resizeKey(value:number,key:string,min:number,max:number,step=10,reverse=false){
 if(key==='Home')return min;if(key==='End')return max;
 if(['ArrowLeft','ArrowUp'].includes(key))return resizeValue(value,reverse?step:-step,min,max);
 if(['ArrowRight','ArrowDown'].includes(key))return resizeValue(value,reverse?-step:step,min,max);
 return null;
}
