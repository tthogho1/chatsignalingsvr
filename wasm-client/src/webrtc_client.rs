use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    console, MediaDevices, MediaStream, MediaStreamConstraints,
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
}

impl WebRTCClient {
    pub async fn new() -> Result<Self, JsValue> {
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
        };

        client.setup_peer_connection_handlers()?;

        Ok(client)
    }

    fn setup_peer_connection_handlers(&self) -> Result<(), JsValue> {
        let remote_stream_ref = self.remote_stream.clone();

        // OnTrack handler - when remote stream is received
        let ontrack_callback = Closure::wrap(Box::new(move |event: web_sys::RtcTrackEvent| {
            console::log_1(&"Received remote track".into());
            
            let streams = event.streams();
            if streams.length() > 0 {
                if let Ok(stream) = streams.get(0).dyn_into::<MediaStream>() {
                    *remote_stream_ref.borrow_mut() = Some(stream.clone());
                    console::log_1(&"Remote stream received and stored".into());
                }
            }
        }) as Box<dyn FnMut(web_sys::RtcTrackEvent)>);
        
        self.peer_connection.set_ontrack(Some(ontrack_callback.as_ref().unchecked_ref()));
        ontrack_callback.forget();

        // OnICECandidate handler
        let pc_clone = self.peer_connection.clone();
        let onicecandidate_callback = Closure::wrap(Box::new(move |event: web_sys::RtcPeerConnectionIceEvent| {
            if let Some(candidate) = event.candidate() {
                console::log_1(&"New ICE candidate generated".into());
                // In a real implementation, this would be sent through the signaling channel
                // For now, we'll log it
                console::log_2(&"ICE candidate:".into(), &candidate.candidate().into());
            }
        }) as Box<dyn FnMut(web_sys::RtcPeerConnectionIceEvent)>);
        
        self.peer_connection.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
        onicecandidate_callback.forget();

        // OnConnectionStateChange handler
        let onconnectionstatechange_callback = Closure::wrap(Box::new(move |_event| {
            console::log_1(&"Connection state changed".into());
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
        let mut constraints = MediaStreamConstraints::new();
        constraints.set_video(&JsValue::from(true));
        constraints.set_audio(&JsValue::from(true));

        // Get user media
        let promise = media_devices.get_user_media_with_constraints(&constraints)?;
        let js_future = JsFuture::from(promise);
        let stream = js_future.await?;
        let media_stream: MediaStream = stream.into();

        // Store local stream
        *self.local_stream.borrow_mut() = Some(media_stream.clone());

        // Add tracks to peer connection
        let tracks = media_stream.get_tracks();
        for i in 0..tracks.length() {
            let track = tracks.get(i);
            if let Ok(media_track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
                // Note: add_track is not available in web-sys, we'll handle this in JavaScript
                // For now, we'll just store the stream
                console::log_1(&"Track added to local stream".into());
            }
        }

        Ok(media_stream)
    }

    pub async fn create_offer(&self) -> Result<String, JsValue> {
        let promise = self.peer_connection.create_offer();
        let js_future = JsFuture::from(promise);
        let offer = js_future.await?;
        let offer_desc: RtcSessionDescription = offer.into();

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
        let mut remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
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
        let mut remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        remote_desc.set_sdp(sdp);

        // Set remote description
        let set_remote_promise = self.peer_connection.set_remote_description(&remote_desc);
        let set_remote_future = JsFuture::from(set_remote_promise);
        set_remote_future.await?;

        Ok(())
    }

    pub async fn handle_ice_candidate(&self, candidate_str: &str) -> Result<(), JsValue> {
        // Parse ICE candidate JSON
        let candidate_data: serde_json::Value = serde_json::from_str(candidate_str)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse ICE candidate: {}", e)))?;

        if let (Some(candidate), Some(sdp_mid)) = (
            candidate_data["candidate"].as_str(),
            candidate_data["sdpMid"].as_str()
        ) {
            let mut ice_candidate_init = RtcIceCandidateInit::new(candidate);
            ice_candidate_init.set_sdp_mid(Some(sdp_mid));
            
            if let Some(sdp_m_line_index) = candidate_data["sdpMLineIndex"].as_u64() {
                ice_candidate_init.set_sdp_m_line_index(Some(sdp_m_line_index as u16));
            }

            let ice_candidate = RtcIceCandidate::new(&ice_candidate_init)?;
            let promise = self.peer_connection.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&ice_candidate));
            let js_future = JsFuture::from(promise);
            js_future.await?;
        }

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
        self.peer_connection.close();
        
        // Stop all tracks
        if let Some(ref stream) = &*self.local_stream.borrow() {
            let tracks = stream.get_tracks();
            for i in 0..tracks.length() {
                let track = tracks.get(i);
                if let Ok(media_track) = track.dyn_into::<web_sys::MediaStreamTrack>() {
                    media_track.stop();
                }
            }
        }

        *self.local_stream.borrow_mut() = None;
        *self.remote_stream.borrow_mut() = None;

        Ok(())
    }

    pub fn get_local_stream(&self) -> Option<MediaStream> {
        self.local_stream.borrow().clone()
    }

    pub fn get_remote_stream(&self) -> Option<MediaStream> {
        self.remote_stream.borrow().clone()
    }
}