import { For, createEffect, createSignal } from 'solid-js'
import './App.css'
import { invoke } from '@tauri-apps/api'
import ProjectCard from './ProjectCard'

function App() {

  const [reloading, setReloading] = createSignal(true)
  const [ids, setIDs] = createSignal<string[]>([])
  createEffect(() => {
    const r = reloading();
    if (!r) return;
    invoke("update_projects").then(() => {
      invoke<string[]>("project_ids").then((res) => setIDs(res)).catch((err) => console.error(err));
    }).catch((err) => console.error(err));

    setReloading(false);
  }, [reloading])
  return (
    <>
      <div>
        <button onClick={() => setReloading(true)} disabled={reloading()}>Refresh</button>
      </div>
      <div class='container'>
        <For each={ids()}>{(id) => <ProjectCard id={id} />}</For>
      </div>
    </>
  )
}

export default App
