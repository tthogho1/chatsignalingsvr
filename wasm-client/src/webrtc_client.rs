use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    console, MediaStream, MediaStreamConstraints,
    RtcPeerConnection, RtcConfiguration, RtcSessionDescription,
    RtcSessionDescriptionInit, RtcIceCandidate, RtcIceCandidateInit, RtcSdpType
};
use js_sys::{Object, Array, Reflect};
use std::rc::Rc;
use std::cell::RefCell;

pub struct WebRTCClient {
    peer_connection: RtcPeerConnection,
    local_stream: Rc<RefCell<Option<MediaStream>>>,
    remote_stream: Rc<RefCell<Option<MediaStream>>>,
    is_camera_on: bool,
    is_mic_on: bool,
    ice_callback_handler: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
    video_chat: Rc<RefCell<crate::video_chat::VideoChat>>, // 追加
}

impl WebRTCClient {
    pub async fn new(video_chat: Rc<RefCell<crate::video_chat::VideoChat>>) -> Result<Self, JsValue> {
        // Create RTCConfiguration
        let config = RtcConfiguration::new();
        
        // Set ICE servers
        let ice_servers = Array::new();
        let stun_server1 = Object::new();
        Reflect::set(&stun_server1, &"urls".into(), &"stun:stun.l.google.com:19302".into())?;
        ice_servers.push(&stun_server1);
        
        let stun_server2 = Object::new();
        Reflect::set(&stun_server2, &"urls".into(), &"stun:stun1.l.google.com:19302".into())?;
        ice_servers.push(&stun_server2);
        
        config.set_ice_servers(&ice_servers);

        // Create peer connection
        let peer_connection = RtcPeerConnection::new_with_configuration(&config)?;

        let client = WebRTCClient {
            peer_connection,
            local_stream: Rc::new(RefCell::new(None)),
            remote_stream: Rc::new(RefCell::new(None)),
            is_camera_on: true,
            is_mic_on: true,
            ice_callback_handler: Rc::new(RefCell::new(None)),
            video_chat: video_chat.clone(), // 追加
        };

        client.setup_peer_connection_handlers(video_chat)?;

        Ok(client)
    }

    fn setup_peer_connection_handlers(&self, video_chat: Rc<RefCell<crate::video_chat::VideoChat>>) -> Result<(), JsValue> {
        let remote_stream_ref = self.remote_stream.clone();

        // OnTrack handler - when remote stream is received
        let video_chat_clone = video_chat.clone();
        let ontrack_callback = Closure::wrap(Box::new(move |event: web_sys::RtcTrackEvent| {
            console::log_1(&"Received remote track".into());
            
            let streams = event.streams();
            if streams.length() > 0 {
                if let Ok(stream) = streams.get(0).dyn_into::<MediaStream>() {
                    *remote_stream_ref.borrow_mut() = Some(stream.clone());
                    console::log_1(&"Remote stream received and stored".into());
                    // 推奨設計: VideoChatのhandle_remote_streamを呼び出す
                    if let Err(e) = (*video_chat_clone.borrow()).handle_remote_stream(&stream) {
                        console::error_2(&"Error in handle_remote_stream:".into(), &e);
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::RtcTrackEvent)>);
        
        self.peer_connection.set_ontrack(Some(ontrack_callback.as_ref().unchecked_ref()));
        ontrack_callback.forget();

        // OnICECandidate handler
        let ice_callback_handler = self.ice_callback_handler.clone();
        let onicecandidate_callback = Closure::wrap(Box::new(move |event: web_sys::RtcPeerConnectionIceEvent| {
            if let Some(candidate) = event.candidate() {
                console::log_1(&"New ICE candidate generated".into());
                console::log_2(&"ICE candidate:".into(), &candidate.candidate().into());
                
                // ICE候補をシグナリングサーバーに送信
                if let Some(ref handler) = ice_callback_handler.borrow().as_ref() {
                    let candidate_str = candidate.candidate();
                    console::log_1(&"✅ Sending ICE candidate to signaling server".into());
                    handler(&candidate_str);
                } else {
                    console::log_1(&"⚠️ ICE candidate generated but no callback handler set".into());
                }
            }
        }) as Box<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>);
        
        self.peer_connection.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
        onicecandidate_callback.forget();

        // OnConnectionStateChange handler
        let onconnectionstatechange_callback = Closure::wrap(Box::new(move |_event| {
            console::log_1(&"WebRTC connection state changed".into());
        }) as Box<dyn FnMut(JsValue)>);
        
        self.peer_connection.set_onconnectionstatechange(Some(onconnectionstatechange_callback.as_ref().unchecked_ref()));
        onconnectionstatechange_callback.forget();

        Ok(())
    }

    pub async fn get_user_media(&self) -> Result<MediaStream, JsValue> {
        let window = web_sys::window().ok_or("No global window exists")?;
        let navigator = window.navigator();
        let media_devices = navigator.media_devices()?;

        // Create constraints
        let constraints = MediaStreamConstraints::new();
        constraints.set_video(&JsValue::from(true));
        constraints.set_audio(&JsValue::from(true));

        // Get user media
        let promise = media_devices.get_user_media_with_constraints(&constraints)?;
        let js_future = JsFuture::from(promise);
        let stream = js_future.await?;
        let media_stream: MediaStream = stream.into();

        // Store local stream
        *self.local_stream.borrow_mut() = Some(media_stream.clone());

        // Add tracks to peer connection using JavaScript
        let tracks = media_stream.get_tracks();
        console::log_2(&"Adding tracks to peer connection, count:".into(), &tracks.length().into());
        
        for i in 0..tracks.length() {
            let track = tracks.get(i);
            if let Ok(media_track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
                console::log_2(&"Adding track:".into(), &media_track.kind().into());
                
                // Use JavaScript to add track (workaround for web-sys limitation)
                let js_code = format!(
                    "arguments[0].addTrack(arguments[1], arguments[2])"
                );
                let function = js_sys::Function::new_with_args(
                    "pc,track,stream",
                    &js_code
                );
                let _ = function.call3(
                    &JsValue::NULL,
                    &self.peer_connection.clone().into(),
                    &media_track.into(),
                    &media_stream.clone().into()
                );
            }
        }

        console::log_1(&"All tracks added to peer connection".into());
        Ok(media_stream)
    }

    pub async fn create_offer(&self) -> Result<String, JsValue> {
        console::log_1(&"Creating WebRTC offer with fresh peer connection".into());
        
        // Check if we have local tracks
        let senders = self.peer_connection.get_senders();
        console::log_2(&"Number of senders (tracks) in peer connection:".into(), &senders.length().into());
        
        let promise = self.peer_connection.create_offer();
        let js_future = JsFuture::from(promise);
        let offer = js_future.await?;
        let offer_desc: RtcSessionDescription = offer.into();

        console::log_2(&"Generated SDP offer length:".into(), &offer_desc.sdp().len().into());
        console::log_2(&"SDP offer preview (200 chars):".into(), &offer_desc.sdp().chars().take(200).collect::<String>().into());

        // Set local description
        let offer_init = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_init.set_sdp(&offer_desc.sdp());
        let set_local_promise = self.peer_connection.set_local_description(&offer_init);
        let set_local_future = JsFuture::from(set_local_promise);
        set_local_future.await?;

        Ok(offer_desc.sdp())
    }

    pub async fn handle_offer(&self, sdp: &str) -> Result<String, JsValue> {
        // Create remote description from offer
        let remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        remote_desc.set_sdp(sdp);

        // Set remote description
        let set_remote_promise = self.peer_connection.set_remote_description(&remote_desc);
        let set_remote_future = JsFuture::from(set_remote_promise);
        set_remote_future.await?;

        // Create answer
        let promise = self.peer_connection.create_answer();
        let js_future = JsFuture::from(promise);
        let answer = js_future.await?;
        let answer_desc: RtcSessionDescription = answer.into();

        // Set local description
        let answer_init = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_init.set_sdp(&answer_desc.sdp());
        let set_local_promise = self.peer_connection.set_local_description(&answer_init);
        let set_local_future = JsFuture::from(set_local_promise);
        set_local_future.await?;

        Ok(answer_desc.sdp())
    }

    pub async fn handle_answer(&self, sdp: &str) -> Result<(), JsValue> {
        // Create remote description from answer
        let remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        remote_desc.set_sdp(sdp);

        // Set remote description
        let set_remote_promise = self.peer_connection.set_remote_description(&remote_desc);
        let set_remote_future = JsFuture::from(set_remote_promise);
        set_remote_future.await?;

        Ok(())
    }

    pub async fn handle_ice_candidate(&self, candidate_str: &str) -> Result<(), JsValue> {
        console::log_1(&"Processing received ICE candidate".into());
        console::log_2(&"Candidate string:".into(), &candidate_str.into());
        
        // Create ICE candidate from string
        let ice_candidate_init = RtcIceCandidateInit::new(candidate_str);
        let ice_candidate = RtcIceCandidate::new(&ice_candidate_init)?;
        
        // Add ICE candidate to peer connection
        let promise = self.peer_connection.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&ice_candidate));
        let js_future = JsFuture::from(promise);
        js_future.await?;
        
        console::log_1(&"ICE candidate added successfully".into());
        Ok(())
    }

    pub async fn toggle_camera(&mut self) -> Result<bool, JsValue> {
        if let Some(ref stream) = &*self.local_stream.borrow() {
            let video_tracks = stream.get_video_tracks();
            for i in 0..video_tracks.length() {
                let track = video_tracks.get(i);
                if let Ok(video_track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
                    self.is_camera_on = !self.is_camera_on;
                    video_track.set_enabled(self.is_camera_on);
                }
            }
        }
        Ok(self.is_camera_on)
    }

    pub async fn toggle_microphone(&mut self) -> Result<bool, JsValue> {
        if let Some(ref stream) = &*self.local_stream.borrow() {
            let audio_tracks = stream.get_audio_tracks();
            for i in 0..audio_tracks.length() {
                let track = audio_tracks.get(i);
                if let Ok(audio_track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
                    self.is_mic_on = !self.is_mic_on;
                    audio_track.set_enabled(self.is_mic_on);
                }
            }
        }
        Ok(self.is_mic_on)
    }

    pub fn close_connection(&self) -> Result<(), JsValue> {
        console::group_1(&"=== Closing WebRTC Connection ===".into());
        
        self.peer_connection.close();
        console::log_1(&"PeerConnection closed".into());
        
        // Stop all tracks
        if let Some(ref stream) = &*self.local_stream.borrow() {
            let tracks = stream.get_tracks();
            console::log_2(&"Stopping tracks, count:".into(), &tracks.length().into());
            for i in 0..tracks.length() {
                let track = tracks.get(i);
                if let Ok(media_track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
                    media_track.stop();
                }
            }
        }

        *self.local_stream.borrow_mut() = None;
        *self.remote_stream.borrow_mut() = None;
        console::log_1(&"Local and remote streams cleared".into());
        console::group_end();

        Ok(())
    }

    pub fn get_local_stream(&self) -> Option<MediaStream> {
        self.local_stream.borrow().clone()
    }

    pub fn get_remote_stream(&self) -> Option<MediaStream> {
        self.remote_stream.borrow().clone()
    }

    pub fn set_ice_candidate_callback<F>(&self, callback: F) 
    where
        F: Fn(&str) + 'static,
    {
        *self.ice_callback_handler.borrow_mut() = Some(Box::new(callback));
        console::log_1(&"✅ ICE candidate callback handler registered".into());
    }
}