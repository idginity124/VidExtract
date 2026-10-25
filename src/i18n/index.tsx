import { createContext, useContext, useEffect, useState } from "react";
import tr from "./tr.json"; import en from "./en.json"; import es from "./es.json"; import fr from "./fr.json"; import de from "./de.json"; import pt from "./pt.json"; import ru from "./ru.json"; import ar from "./ar.json"; import hi from "./hi.json"; import zh from "./zh.json";
export const languages = [
  {value:"tr",label:"Türkçe"},{value:"en",label:"English"},{value:"es",label:"Español"},{value:"fr",label:"Français"},{value:"de",label:"Deutsch"},
  {value:"pt",label:"Português"},{value:"ru",label:"Русский"},{value:"ar",label:"العربية"},{value:"hi",label:"हिन्दी"},{value:"zh",label:"中文"},
] as const;
export type Locale = typeof languages[number]["value"];
const catalogs:Record<Locale,Record<string,string>>={tr,en,es,fr,de,pt,ru,ar,hi,zh};
export function translate(locale:Locale,key:string,values:Record<string,string|number>={}) {
  const text=catalogs[locale][key] ?? en[key as keyof typeof en] ?? key;
  return text.replace(/\{(\w+)\}/g,(match,name)=>values[name]!==undefined?typeof values[name]==="number"?new Intl.NumberFormat(locale,{maximumFractionDigits:1}).format(values[name] as number):String(values[name]):match);
}
export function initialLocale():Locale {
  const saved=localStorage.getItem("ve-locale");
  if(languages.some(l=>l.value===saved))return saved as Locale;
  const system=navigator.language.split("-")[0];
  return languages.some(l=>l.value===system)?system as Locale:"en";
}
interface LocaleContextValue {locale:Locale;setLocale:(value:Locale)=>void;t:(key:string,values?:Record<string,string|number>)=>string}
const Context=createContext<LocaleContextValue>({locale:"en",setLocale:()=>{},t:(key)=>key});
export function LocaleProvider({children}:{children:React.ReactNode}) {
  const [locale,setLocale]=useState<Locale>(initialLocale);
  useEffect(()=>{document.documentElement.lang=locale;document.documentElement.dir=locale==="ar"?"rtl":"ltr";localStorage.setItem("ve-locale",locale);},[locale]);
  return <Context.Provider value={{locale,setLocale,t:(key,values)=>translate(locale,key,values)}}>{children}</Context.Provider>;
}
export const useLocale=()=>useContext(Context);
export function errorKey(error:unknown) {
  const text=String(error);
  for(const [code,key] of Object.entries({TRANSLATION_NETWORK:"errorTranslationNetwork",TRANSLATION_QUOTA:"errorTranslationQuota",TRANSLATION_TOO_LONG:"errorTranslationTooLong",TRANSLATION_EMAIL:"errorTranslationEmail",TRANSLATION_RESPONSE:"errorTranslationResponse",TRANSLATION_MODEL_REQUIRED:"errorTranslationModel",TRANSLATION_OFFLINE_LANGUAGE:"translationOfflineUnsupported"})){if(text.includes(code))return key;}
  if(text.includes("INSUFFICIENT_MEMORY"))return "errorMemory";
  if(text.includes("İptal edildi"))return "cancelled";
  if(text.includes("Önce")&&text.includes("model"))return "errorModel";
  if(text.includes("Araç başlatılamadı")||text.includes("No such file"))return "errorEngine";
  return "errorTitle";
}
