import { invoke } from "@tauri-apps/api"
import { Accessor, For, Show, createEffect, createSignal, onMount } from "solid-js"
import "./ProjectCard.css"
import { nameFormat } from "./utils"

export default (props: {
  id: string 
}) => {

  const {id} = props;
  
  return <div class="project-card">
    <NameEl id={id} />
    {/* <span>{remoteName()}</span>|<span>{localName()}</span> */}
    <RemoteEl id={id} />
  </div>
}

const RemoteEl = (props: {
  id: string,
}) => {

  const {id} = props;
  const [commits, setLocalCommits] = createSignal<{[b: string]: string}>({});

  onMount(() => {
    invoke<{[b: string]: string}>("project_local_commits", {id: id})
      .then((res) => setLocalCommits(res))
      .catch((err) => console.log(err));
  })
  return <div class="local-el">
    <div>Branch:</div>
    <div>
      <For each={Object.entries(commits())}>
        {([branch, commitid]) => <div>{branch}: {commitid}</div>}
      </For>
    </div>
  </div>
}  

const NameEl = (props: {
  id: string,
}) => {

  const {id} = props;
  const [remoteName, setRemoteName] = createSignal<string>();
  const [localName, setLocalName] = createSignal<string>();

  onMount(() => {
    invoke<string | undefined>("project_remote_name", {id: id})
      .then((res) => setRemoteName(res ? nameFormat(res) : undefined))
      .catch((err) => console.log(err));
    invoke<string | undefined>("project_local_name", {id: id})
      .then((res) => setLocalName(res ? nameFormat(res) : undefined))
      .catch((err) => console.log(err));
  })

  return <div class="name-el">
    <Show when={remoteName()}>
      <span>{remoteName()}</span>
    </Show>
    <Show when={localName() && (localName() !== remoteName())}>
      <span>{localName()}</span>
    </Show>
    <Show when={!localName() && !remoteName()}>
      <span>Unknown</span>
    </Show>
  </div>
}
