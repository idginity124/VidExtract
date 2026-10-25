import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown } from "lucide-react";
export interface Option { value:string;label:string;detail?:string }
export default function Select({label,value,options,onChange,disabled=false,compact=false}:{label:string;value:string;options:readonly Option[];onChange:(value:string)=>void;disabled?:boolean;compact?:boolean}) {
  const [open,setOpen]=useState(false);const [active,setActive]=useState(0);
  const [position,setPosition]=useState<React.CSSProperties>({visibility:"hidden"});
  const button=useRef<HTMLButtonElement>(null);const menu=useRef<HTMLDivElement>(null);const id=useId();
  const chosen=options.find(option=>option.value===value);
  function show(){if(disabled)return;setPosition({visibility:"hidden"});setActive(Math.max(0,options.findIndex(o=>o.value===value)));setOpen(true);}
  function choose(index:number){const option=options[index];if(option)onChange(option.value);setOpen(false);button.current?.focus();}
  useLayoutEffect(()=>{
    if(!open||!button.current)return;
    function reposition(){
      if(!button.current)return;
      const rect=button.current.getBoundingClientRect();
      if(rect.bottom<0||rect.top>window.innerHeight){setOpen(false);return;}
      const width=Math.min(Math.max(rect.width,compact?210:220),window.innerWidth-24);
      const availableBelow=window.innerHeight-rect.bottom-16;const above=availableBelow<180&&rect.top>availableBelow;
      const maxHeight=Math.max(70,Math.min(296,above?rect.top-20:availableBelow));
      // Anchor the popup by its near edge; options can have different real heights.
      const next:React.CSSProperties={visibility:"visible",left:Math.max(12,Math.min(document.dir==="rtl"?rect.right-width:rect.left,window.innerWidth-width-12)),width,maxHeight};
      if(above)next.bottom=window.innerHeight-rect.top+7;else next.top=rect.bottom+7;
      setPosition(current=>Object.keys(next).length===Object.keys(current).length&&Object.entries(next).every(([key,value])=>current[key as keyof React.CSSProperties]===value)?current:next);
    }
    reposition();
    function scrolled(event:Event){if(!menu.current?.contains(event.target as Node))reposition();}
    document.addEventListener("scroll",scrolled,true);window.addEventListener("resize",reposition);
    return()=>{document.removeEventListener("scroll",scrolled,true);window.removeEventListener("resize",reposition);};
  },[open,compact,options.length]);
  useEffect(()=>{
    if(!open)return;
    function outside(event:PointerEvent){if(!button.current?.contains(event.target as Node)&&!menu.current?.contains(event.target as Node))setOpen(false);}
    document.addEventListener("pointerdown",outside);
    return()=>document.removeEventListener("pointerdown",outside);
  },[open]);
  useEffect(()=>{
    if(!open||!menu.current)return;
    const option=menu.current.querySelector<HTMLElement>(`[data-index="${active}"]`);if(!option)return;
    const top=option.offsetTop,bottom=top+option.offsetHeight;
    if(top<menu.current.scrollTop)menu.current.scrollTop=top-5;
    else if(bottom>menu.current.scrollTop+menu.current.clientHeight)menu.current.scrollTop=bottom-menu.current.clientHeight+5;
  },[active,open]);
  function keyboard(event:React.KeyboardEvent){
    if(["ArrowDown","ArrowUp","Home","End","Escape","Enter"," "].includes(event.key))event.preventDefault();
    if(event.key==="Escape"){setOpen(false);button.current?.focus();return;}
    if(!open){if(["ArrowDown","ArrowUp","Enter"," "].includes(event.key))show();return;}
    if(event.key==="ArrowDown")setActive(i=>(i+1)%options.length);
    if(event.key==="ArrowUp")setActive(i=>(i-1+options.length)%options.length);
    if(event.key==="Home")setActive(0);if(event.key==="End")setActive(options.length-1);
    if(event.key==="Enter"||event.key===" ")choose(active);if(event.key==="Tab")setOpen(false);
  }
  return <div className={`custom-select ${compact?"compact":""}`} onKeyDown={keyboard}>
    {!compact&&<span className="field-label">{label}</span>}
    <button ref={button} type="button" className={`select-trigger ${open?"is-open":""}`} aria-label={label} aria-haspopup="listbox" aria-expanded={open} aria-controls={open?id:undefined} aria-activedescendant={open?`${id}-${active}`:undefined} disabled={disabled} onClick={()=>open?setOpen(false):show()}><span>{chosen?.label||value||label}</span><ChevronDown size={15}/></button>
    {open&&createPortal(<div id={id} ref={menu} role="listbox" aria-label={label} className="select-menu" style={position}>{options.map((option,index)=><div key={option.value} id={`${id}-${index}`} role="option" aria-selected={value===option.value} data-index={index} className={`select-option ${active===index?"focused":""}`} onPointerMove={()=>setActive(index)} onClick={()=>choose(index)}><span><strong>{option.label}</strong>{option.detail&&<small>{option.detail}</small>}</span>{value===option.value&&<Check size={16}/>}</div>)}</div>,document.body)}
  </div>;
}
