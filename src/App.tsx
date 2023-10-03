import { For, createSignal } from 'solid-js'
import solidLogo from './assets/solid.svg'
import viteLogo from '/vite.svg'
import './App.css'
import { invoke } from '@tauri-apps/api'
import ProjectCard from './ProjectCard'

function App() {
  const [count, setCount] = createSignal(0)
  const [ids, setIDs] = createSignal<string[]>([])
  invoke("update_projects").then(() => {
    invoke<string[]>("project_ids").then((res) => setIDs(res)).catch((err) => console.error(err));
  }).catch((err) => console.error(err));
  return (
    <>
      <div class='container'>
        <For each={ids()}>{(id) => <ProjectCard id={id} />}</For>
      </div>
    </>
  )
}

export default App
