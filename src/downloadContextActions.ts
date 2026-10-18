export function contextTaskIds(clickedId:string,selected:Set<string>):string[]{
 return selected.has(clickedId)?[...selected]:[clickedId];
}
