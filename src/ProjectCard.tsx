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

type Relation = "Ahead" | "Behind" | "Same" | "Null"
const RemoteEl = (props: {
  id: string,
}) => {

  const {id} = props;
  const [current, setCurrent] = createSignal<{[b: string]: string}>({});
  const [relations, setRelations] = createSignal<{[b: string]: Relation}>({});

  onMount(() => {
    invoke<{[b: string]: string}>("project_local_commits", {id: id})
      .then((current) => {
        setCurrent(current)
      })
      .catch((err) => console.log(err));
  })

  createEffect(() => {
    const c = current();
    if (c === null) return;
    const nrelations: {[b: string]: Relation} = {};
    let promises = [];
    for (const [branch, cc] of Object.entries(c)) {
      promises.push(
      invoke<Relation>("branch_relation", {id: id, branch: branch, current: cc})
        .then((rel) => nrelations[branch] = rel)
        .catch((err) => console.log(err)));
    }
    Promise.all(promises).then(() => setRelations(nrelations));
  }, [current])

  return <div class="local-el">
    <div>
      <For each={Object.entries(relations())}>
        {([branch, rel]) => <div>{branch}: {rel}</div>}
      </For>
    </div>
  </div>
}

const NumChangesEl = (props: {
  id: string,
}) => {

  const {id} = props;
  const [numChanges, setNumChanges] = createSignal<number>();
  const [status, setStatus] = createSignal<"unknown" | "no" | "yes">("unknown");
  onMount(() => {
    invoke<number>("project_changes", {id: id})
      .then((res) => setNumChanges(res))
      .catch((err) => console.log(err));
  })

  createEffect(() => {
    const n = numChanges();
    if (n === undefined) setStatus("unknown");
    else if (n === 0) setStatus("no");
    else setStatus("yes");
  }, [numChanges])

  return <div class={`has-changes ${status()}`}></div>
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
    <Show when={localName()}>
        <span>{localName()}</span>
        <NumChangesEl id={id} />
    </Show>
    <Show when={!localName() && !remoteName()}>
      <span>Unknown</span>
    </Show>
  </div>
}
