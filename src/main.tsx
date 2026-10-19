import React from 'react';
import {createRoot} from 'react-dom/client';
import App from './App';
import './style.css';

class RecoveryBoundary extends React.Component<React.PropsWithChildren, {error:string|null}> {
  state:{error:string|null}={error:null};
  static getDerivedStateFromError(error:Error){return {error:error.message||String(error)}}
  componentDidCatch(error:Error,info:React.ErrorInfo){
    console.error('Hellforge rendering failure',error,info.componentStack);
    try{localStorage.setItem('hellforge-last-ui-error',`${new Date().toISOString()}\n${error.stack||error.message}\n${info.componentStack}`)}catch{}
  }
  render(){if(this.state.error)return <main style={{padding:40,color:'#eee',background:'#141416',minHeight:'100vh',fontFamily:'sans-serif'}}><h1>Hellforge — UI recovery</h1><p>Інтерфейс зіткнувся з помилкою. Дані бібліотеки не видалено.</p><pre style={{whiteSpace:'pre-wrap',color:'#ff917e'}}>{this.state.error}</pre><button onClick={()=>window.location.reload()} style={{padding:12,cursor:'pointer'}}>Перезапустити інтерфейс</button></main>;return this.props.children}
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><RecoveryBoundary><App/></RecoveryBoundary></React.StrictMode>);
