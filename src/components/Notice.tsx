import { Check, CircleAlert, X } from "lucide-react";
import { errorKey, useLocale } from "../i18n";
export default function Notice({error,message,onClose}:{error?:unknown;message?:string;onClose?:()=>void}) {
  const {t}=useLocale();const isError=Boolean(error);
  if(!error&&!message)return null;
  return <div role={isError?"alert":"status"} className={`notice ${isError?"is-error":"is-success"}`}>{isError?<CircleAlert size={18}/>:<Check size={18}/>}<div><span>{isError?t(errorKey(error)):message}</span></div>{onClose&&<button className="icon-button" aria-label={t("close")} onClick={onClose}><X size={16}/></button>}</div>;
}
