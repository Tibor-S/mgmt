import { invoke } from "@tauri-apps/api"
import { createSignal, onMount } from "solid-js"


export default (props: {
  id: string 
}) => {

  const {id} = props;

  const [remoteName, setRemoteName] = createSignal<string>();
  const [localName, setLocalName] = createSignal<string>();

  onMount(() => {
    invoke<string | undefined>("project_remote_name", {id: id})
      .then((res) => setRemoteName(res))
      .catch((err) => console.log(err));
      invoke<string | undefined>("project_local_name", {id: id})
        .then((res) => setLocalName(res))
        .catch((err) => console.log(err));
  })

  return <div>
    <span>{remoteName()}</span>|<span>{localName()}</span>
  </div>
}