//! Owner-started quest progress jobs. No background work before a deliberate click.
use client_core::auth::SessionSecret;
use discord_api::{
	DiscordApi,
	proxy::ApiProxy,
	quests::{MAX_RUN_SECONDS, Quest, Task, VoiceContext},
};
use eframe::egui;
use model::Id;
use std::{
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::{
	runtime::Runtime,
	sync::{mpsc, watch},
	task::JoinHandle,
};

pub(super) struct Session {
	pub secret: Option<Arc<SessionSecret>>,
	pub generation: u64,
	pub offline: bool,
	pub proxy: watch::Receiver<Option<ApiProxy>>,
	pub voice: Option<VoiceContext>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
	generation: u64,
	account: usize,
}
impl Session {
	fn key(&self) -> Key {
		Key {
			generation: self.generation,
			account: self.secret.as_ref().map_or(0, |s| Arc::as_ptr(s) as usize),
		}
	}
}
struct Update {
	quests: Option<Vec<Quest>>,
	note: String,
	done: bool,
}
struct Job {
	task: JoinHandle<()>,
	updates: mpsc::Receiver<Update>,
	voice: Option<VoiceContext>,
}
#[derive(Clone, Copy)]
enum Action {
	Refresh,
	Run(Id),
	RunAll,
	Claim(Id),
}

pub(super) struct Quests {
	pub open: bool,
	key: Option<Key>,
	quests: Vec<Quest>,
	job: Option<Job>,
	note: String,
	auto_enroll: bool,
	auto_claim: bool,
	pub pill_dismissed: bool,
}
impl Default for Quests {
	fn default() -> Self {
		Self {
			open: false,
			key: None,
			quests: Vec::new(),
			job: None,
			note: "Click Refresh or Auto Complete All to start.".into(),
			auto_enroll: true,
			auto_claim: true,
			pill_dismissed: false,
		}
	}
}
impl Drop for Quests {
	fn drop(&mut self) {
		if let Some(job) = self.job.take() {
			job.task.abort();
		}
	}
}
impl Quests {
	fn stop(&mut self, note: &str) {
		if let Some(job) = self.job.take() {
			job.task.abort();
		}
		self.note = note.into();
	}
	fn observe_session(&mut self, session: &Session) {
		let key = session.key();
		if self.key != Some(key) || session.offline || session.secret.is_none() {
			self.stop("Session changed. Refresh after signing in; any in-flight write may already have reached Discord.");
			self.quests.clear();
			self.key = Some(key);
		} else if self
			.job
			.as_ref()
			.is_some_and(|job| job.voice.is_some() && job.voice != session.voice)
		{
			self.stop(
				"Voice channel changed. Automation stopped; refresh to verify any in-flight result.",
			);
		}
	}
	pub fn show(&mut self, ctx: &egui::Context, runtime: &Runtime, session: Session) {
		self.observe_session(&session);
		let mut done = false;
		if let Some(job) = &mut self.job {
			while let Ok(update) = job.updates.try_recv() {
				if let Some(quests) = update.quests {
					self.quests = quests;
				}
				self.note = update.note;
				done |= update.done;
			}
			if job.task.is_finished() && job.updates.is_empty() {
				done = true;
			}
		}
		if done {
			self.job = None;
		}
		if self.job.is_some() {
			ctx.request_repaint_after(Duration::from_millis(100));
		}
		let running = self.job.is_some();
		if !self.pill_dismissed || running {
			egui::Area::new(egui::Id::unique("rustcord-quests-button"))
				.anchor(egui::Align2::RIGHT_BOTTOM, [-20.0, -32.0])
				.order(egui::Order::Foreground)
				.movable(true)
				.show(ctx, |ui| {
					let colors = ui::design::palette_for(ctx);
					let radius = ui::design::effective_widget_radius() as f32;
					ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
					let border_color = if running {
						colors.positive
					} else {
						colors.border
					};
					let mut hide_clicked = false;
					let frame_resp = egui::Frame::new()
						.fill(colors.raised)
						.stroke(egui::Stroke::new(1.0, border_color))
						.corner_radius(radius as u8)
						.inner_margin(egui::Margin::symmetric(10, 6))
						.show(ui, |ui| {
							ui.horizontal(|ui| {
								ui.spacing_mut().item_spacing.x = 6.0;
								let icon_text = if running { "⚡" } else { "🎯" };
								ui.label(
									egui::RichText::new(icon_text)
										.color(if running {
											colors.positive
										} else {
											colors.accent
										})
										.size(13.0)
										.strong(),
								);
								ui.label(
									egui::RichText::new("Quests")
										.color(colors.text_strong)
										.size(13.0)
										.strong(),
								);
								if running {
									egui::Frame::new()
										.fill(colors.positive.gamma_multiply(0.2))
										.corner_radius(4)
										.inner_margin(egui::Margin::symmetric(5, 1))
										.show(ui, |ui| {
											ui.label(
												egui::RichText::new("ACTIVE")
													.color(colors.positive)
													.size(10.0)
													.strong(),
											);
										});
								}
								ui.add_space(4.0);
								let close_btn = ui
									.add(
										egui::Label::new(
											egui::RichText::new("×").color(colors.muted).size(15.0),
										)
										.sense(egui::Sense::click()),
									)
									.on_hover_text(
										"Hide Quests badge (open anytime from Home context menu)",
									);
								if close_btn.clicked() {
									hide_clicked = true;
								}
							});
						});
					if hide_clicked {
						self.pill_dismissed = true;
					} else {
						let response = ui.interact(
							frame_resp.response.rect,
							ui.scope_id().with("quest-pill-click"),
							egui::Sense::click(),
						);
						if response.clicked() {
							self.open = !self.open;
						}
						response.on_hover_text("Open Discord Quests runner (drag to reposition)");
					}
				});
		}
		if !self.open {
			return;
		}
		let mut open = self.open;
		let mut action = None;
		let mut stop = false;
		let colors = ui::design::palette_for(ctx);
		let radius = ui::design::effective_widget_radius() as f32;
		let font_id = egui::FontId::new(15.0, ui::design::semibold_family(ctx));
		let title = egui::RichText::new("Discord Quests")
			.font(font_id)
			.color(colors.text_strong);

		egui::Window::new(title)
			.id(egui::Id::unique("rustcord-quests"))
			.open(&mut open)
			.default_width(640.0)
			.default_height(520.0)
			.resizable(true)
			.frame(
				egui::Frame::window(&ctx.style_of(ctx.theme()))
					.fill(colors.chat)
					.stroke(egui::Stroke::new(1.0, colors.border))
					.corner_radius(radius as u8),
			)
			.show(ctx, |ui| {
				egui::Frame::new()
					.fill(colors.raised)
					.corner_radius(radius as u8)
					.inner_margin(10)
					.stroke(egui::Stroke::new(1.0, colors.border))
					.show(ui, |ui| {
						ui.horizontal(|ui| {
							let available =
								!session.offline && session.secret.is_some() && self.job.is_none();
							if ui
								.add_enabled(
									available,
									egui::Button::new(
										egui::RichText::new("🔄 Refresh").color(colors.text),
									)
									.corner_radius(radius as u8),
								)
								.clicked()
							{
								action = Some(Action::Refresh);
							}
							if ui
								.add_enabled(
									available,
									egui::Button::new(
										egui::RichText::new("⚡ Auto Complete All")
											.color(colors.accent_text),
									)
									.fill(colors.accent)
									.corner_radius(radius as u8),
								)
								.clicked()
							{
								action = Some(Action::RunAll);
							}
							if ui
								.add_enabled(
									self.job.is_some(),
									egui::Button::new(
										egui::RichText::new("⏹ Stop").color(egui::Color32::WHITE),
									)
									.fill(colors.danger)
									.corner_radius(radius as u8),
								)
								.clicked()
							{
								stop = true;
							}
							ui.with_layout(
								egui::Layout::right_to_left(egui::Align::Center),
								|ui| {
									ui.checkbox(&mut self.auto_claim, "Auto claim");
									ui.checkbox(&mut self.auto_enroll, "Auto enroll");
								},
							);
						});
						if session.offline || session.secret.is_none() {
							ui.label(
								egui::RichText::new("Sign in to load quests.")
									.color(colors.warning),
							);
						} else if !self.note.is_empty() {
							ui.add_space(4.0);
							ui.label(
								egui::RichText::new(&self.note)
									.size(12.0)
									.color(colors.muted),
							);
						}
					});

				ui.add_space(8.0);

				egui::ScrollArea::vertical()
					.max_height(600.0)
					.show(ui, |ui| {
						if self.quests.is_empty() {
							ui.vertical_centered(|ui| {
								ui.add_space(30.0);
								ui.label(
									egui::RichText::new(
										"No quests loaded. Click Refresh to fetch active quests.",
									)
									.color(colors.muted),
								);
							});
						}
						for quest in &self.quests {
							let progress_frac = (quest.status.value as f32
								/ quest.target.max(1) as f32)
								.clamp(0.0, 1.0);
							egui::Frame::new()
								.fill(colors.raised)
								.corner_radius(radius as u8)
								.inner_margin(10)
								.stroke(egui::Stroke::new(1.0, colors.border))
								.show(ui, |ui| {
									ui.horizontal(|ui| {
										ui.label(
											egui::RichText::new(&quest.name)
												.font(egui::FontId::new(
													14.0,
													ui::design::semibold_family(ctx),
												))
												.color(colors.text_strong),
										);
										ui.with_layout(
											egui::Layout::right_to_left(egui::Align::Center),
											|ui| {
												let (status_label, status_color) =
													if quest.status.claimed {
														("✓ Claimed", colors.positive)
													} else if quest.status.completed {
														("✓ Completed", colors.positive)
													} else if quest.status.enrolled {
														("Enrolled", colors.accent)
													} else {
														("Not enrolled", colors.muted)
													};
												ui.label(
													egui::RichText::new(status_label)
														.color(status_color),
												);
												ui.label(
													egui::RichText::new(format!(
														"• {}",
														quest.task.label()
													))
													.size(12.0)
													.color(colors.muted),
												);
											},
										);
									});

									ui.add_space(4.0);
									ui.horizontal(|ui| {
										let bar = egui::ProgressBar::new(progress_frac)
											.fill(if quest.status.completed {
												colors.positive
											} else {
												colors.accent
											})
											.text(format!(
												"{:.0} / {} ({:.0}%)",
												quest.status.value,
												quest.target,
												progress_frac * 100.0
											));
										ui.add(bar);
									});

									ui.add_space(4.0);
									ui.horizontal(|ui| {
										if !quest.expires.is_empty() {
											ui.label(
												egui::RichText::new(format!(
													"Expires: {}",
													quest.expires
												))
												.size(11.0)
												.color(colors.muted),
											);
										}
										let active = self.job.is_some();
										ui.with_layout(
											egui::Layout::right_to_left(egui::Align::Center),
											|ui| {
												if quest.status.completed && !quest.status.claimed {
													if ui
														.add_enabled(
															!active,
															egui::Button::new(
																egui::RichText::new(
																	"🎁 Claim Reward",
																)
																.color(egui::Color32::WHITE),
															)
															.fill(colors.positive)
															.corner_radius(radius as u8),
														)
														.clicked()
													{
														action = Some(Action::Claim(quest.id));
													}
												} else if !quest.status.completed
													&& quest.task != Task::Unsupported
												{
													let allowed = !active
														&& (!quest.task.needs_voice()
															|| session.voice.is_some());
													if ui
														.add_enabled(
															allowed,
															egui::Button::new(
																egui::RichText::new("▶ Start")
																	.color(colors.accent_text),
															)
															.fill(colors.accent)
															.corner_radius(radius as u8),
														)
														.clicked()
													{
														action = Some(Action::Run(quest.id));
													}
												}
												if quest.task == Task::Unsupported {
													ui.label(
														egui::RichText::new("Console-only quest")
															.size(11.0)
															.color(colors.muted),
													);
												} else if quest.task.needs_voice()
													&& session.voice.is_none()
												{
													ui.label(
														egui::RichText::new(
															"Join voice channel first",
														)
														.size(11.0)
														.color(colors.warning),
													);
												}
											},
										);
									});
								});
							ui.add_space(6.0);
						}
					});
			});
		self.open = open;
		if !open && self.job.is_some() {
			self.stop(
				"Quests window closed. Future requests stopped; refresh to verify any in-flight result.",
			);
		}
		if stop {
			self.stop(
				"Stopped. An in-flight write may already have reached Discord; refresh to verify.",
			);
		} else if let Some(action) = action {
			self.start(ctx, runtime, session, action);
		}
	}
	fn start(&mut self, ctx: &egui::Context, runtime: &Runtime, session: Session, action: Action) {
		if self.job.is_some() || session.offline {
			return;
		}
		let Some(secret) = session.secret else {
			return;
		};
		let api = match DiscordApi::with_proxy(secret, session.proxy) {
			Ok(api) => api,
			Err(error) => {
				self.note = error.label().into();
				return;
			}
		};
		let (send, updates) = mpsc::channel(1);
		let wake = ctx.clone();
		let enroll = self.auto_enroll;
		let claim = self.auto_claim;
		let voice = session.voice;
		self.pill_dismissed = false;
		self.note = "Loading server state…".into();
		let task = runtime.spawn(async move {
			let result = tokio::time::timeout(
				Duration::from_secs(u64::from(MAX_RUN_SECONDS)),
				run(&api, action, voice, enroll, claim, &send, &wake),
			)
			.await;
			let note = match result {
				Ok(Ok(note)) => note,
				Ok(Err(note)) => note,
				Err(_) => {
					"Stopped after 25 minutes. Completion is unconfirmed; refresh before retrying."
						.into()
				}
			};
			let _ = send
				.send(Update {
					quests: None,
					note,
					done: true,
				})
				.await;
			wake.request_repaint();
		});
		self.job = Some(Job {
			task,
			updates,
			voice: match action {
				Action::Run(_) => voice,
				_ => None,
			},
		});
	}
}
fn find(list: &[Quest], id: Id) -> Result<Quest, String> {
	list.iter()
		.find(|q| q.id == id)
		.cloned()
		.ok_or_else(|| "The quest is no longer available. Refresh the list.".into())
}
async fn send_update(
	send: &mpsc::Sender<Update>,
	ctx: &egui::Context,
	list: &[Quest],
	note: String,
) -> Result<(), String> {
	send.send(Update {
		quests: Some(list.to_vec()),
		note,
		done: false,
	})
	.await
	.map_err(|_| "Quest window closed.".to_owned())?;
	ctx.request_repaint();
	Ok(())
}
#[derive(Clone, Copy)]
struct QuestFlags {
	auto_enroll: bool,
	auto_claim: bool,
}

async fn run(
	api: &DiscordApi,
	action: Action,
	voice: Option<VoiceContext>,
	auto_enroll: bool,
	auto_claim: bool,
	send: &mpsc::Sender<Update>,
	ctx: &egui::Context,
) -> Result<String, String> {
	let mut list = api.quests().await.map_err(|e| e.label().to_owned())?;
	send_update(send, ctx, &list, "Server state loaded.".into()).await?;
	let flags = QuestFlags {
		auto_enroll,
		auto_claim,
	};
	match action {
		Action::Refresh => Ok(
			"Server state loaded. Select a quest and click Start, or click Auto Complete All."
				.into(),
		),
		Action::Run(id) => run_single_quest(api, id, voice, flags, send, ctx, &mut list).await,
		Action::RunAll => {
			let uncompleted: Vec<Id> = list
				.iter()
				.filter(|q| {
					(!q.status.completed || !q.status.claimed)
						&& q.task != Task::Unsupported
						&& (!q.task.needs_voice() || voice.is_some())
				})
				.map(|q| q.id)
				.collect();
			if uncompleted.is_empty() {
				return Ok("All eligible quests are already completed and claimed.".into());
			}
			let total = uncompleted.len();
			let all_flags = QuestFlags {
				auto_enroll: true,
				auto_claim: true,
			};
			for (idx, qid) in uncompleted.into_iter().enumerate() {
				let _ = send_update(
					send,
					ctx,
					&list,
					format!("Running quest {} of {}...", idx + 1, total),
				)
				.await;
				if let Err(err) =
					run_single_quest(api, qid, voice, all_flags, send, ctx, &mut list).await
				{
					let _ = send_update(
						send,
						ctx,
						&list,
						format!("Quest failed: {err}; continuing to next..."),
					)
					.await;
				}
			}
			Ok("Auto Complete All finished! State reloaded.".into())
		}
		Action::Claim(id) => {
			let quest = find(&list, id)?;
			api.claim_quest(&quest)
				.await
				.map_err(|e| e.label().to_owned())?;
			list = api.quests().await.map_err(|e| e.label().to_owned())?;
			let claimed = find(&list, id)?.status.claimed;
			send_update(
				send,
				ctx,
				&list,
				"Claim request finished; state reloaded.".into(),
			)
			.await?;
			Ok(if claimed {
				"Reward claim confirmed by Discord."
			} else {
				"Claim was sent, but Discord has not confirmed it. Inspect the official client before retrying."
			}
			.into())
		}
	}
}
async fn run_single_quest(
	api: &DiscordApi,
	id: Id,
	voice: Option<VoiceContext>,
	flags: QuestFlags,
	send: &mpsc::Sender<Update>,
	ctx: &egui::Context,
	list: &mut Vec<Quest>,
) -> Result<String, String> {
	let mut quest = find(list, id)?;
	if quest.status.completed {
		if flags.auto_claim && !quest.status.claimed {
			api.claim_quest(&quest)
				.await
				.map_err(|e| format!("Reward claim failed: {}", e.label()))?;
			*list = api.quests().await.map_err(|e| e.label().to_owned())?;
			send_update(
				send,
				ctx,
				list,
				format!("Claimed reward for {}!", quest.name),
			)
			.await?;
		}
		return Ok(format!("Quest {} was already completed.", quest.name));
	}
	if quest.task == Task::Unsupported {
		return Err("This task is console-only and not supported on desktop; skipping.".into());
	}
	if quest.task.needs_voice() && voice.is_none() {
		return Err("Join a voice channel before starting this activity task.".into());
	}
	if !quest.status.enrolled && !quest.status.completed {
		if !flags.auto_enroll {
			return Err("Enrollment is required. Enable Auto enroll or enroll in Discord.".into());
		}
		api.enroll_quest(&quest)
			.await
			.map_err(|e| e.label().to_owned())?;
		*list = api.quests().await.map_err(|e| e.label().to_owned())?;
		quest = find(list, id)?;
		if !quest.status.enrolled {
			return Err(
				"Enrollment was sent but not confirmed by Discord; refresh before retrying.".into(),
			);
		}
	}
	let start = Instant::now();
	let initial = quest.status.value;
	let mut stalled = 0;
	while !quest.status.completed {
		tokio::time::sleep(Duration::from_secs(if quest.task == Task::Video {
			8
		} else {
			20
		}))
		.await;
		let before = quest.status.value;
		let status_res = if quest.task == Task::Video {
			let timestamp = video_timestamp(initial, start.elapsed(), quest.target);
			api.quest_video_progress(&quest, timestamp).await
		} else {
			api.quest_heartbeat(&quest, voice, false).await
		};
		let mut status = match status_res {
			Ok(s) => s,
			Err(e) => {
				if quest.task != Task::Video {
					return Err(format!(
						"\"{}\" requires active gameplay detection by Discord desktop client (heartbeat returned: {}). Video quests can be automated directly.",
						quest.name,
						e.label()
					));
				}
				return Err(e.label().to_owned());
			}
		};
		status.enrolled |= quest.status.enrolled;
		quest.status = status;
		if quest.status.value >= f64::from(quest.target) {
			quest.status.completed = true;
			if quest.task != Task::Video {
				let _ = api.quest_heartbeat(&quest, voice, true).await;
			}
			if let Ok(updated_list) = api.quests().await {
				*list = updated_list;
				if let Ok(server) = find(list, id) {
					quest.status.completed |= server.status.completed;
					quest.status.claimed |= server.status.claimed;
				}
			}
			if let Some(q) = list.iter_mut().find(|q| q.id == id) {
				*q = quest.clone();
			}
			send_update(
				send,
				ctx,
				list,
				format!(
					"Finished {}. Discord credited {:.0} / {}.",
					quest.name, quest.status.value, quest.target
				),
			)
			.await?;
			break;
		}
		if quest.status.value <= before && !quest.status.completed {
			stalled += 1;
		} else {
			stalled = 0;
		}
		if let Some(q) = list.iter_mut().find(|q| q.id == id) {
			*q = quest.clone();
		}
		send_update(
			send,
			ctx,
			list,
			format!(
				"Running {}. Discord credited {:.0} / {}. Stop cancels further requests.",
				quest.name, quest.status.value, quest.target
			),
		)
		.await?;
		if stalled >= 10 {
			return Err("Discord reported no new progress for 10 checks. Stopped; no completion was assumed.".into());
		}
	}
	// Completion and claims are never inferred from a progress bar reaching its target.
	if flags.auto_claim && !quest.status.claimed {
		api.claim_quest(&quest)
			.await
			.map_err(|e| format!("Quest completed; reward claim failed: {}", e.label()))?;
		*list = api.quests().await.map_err(|e| e.label().to_owned())?;
		let claimed = find(list, id)?.status.claimed;
		send_update(
			send,
			ctx,
			list,
			"Quest completed; reward state reloaded.".into(),
		)
		.await?;
		if !claimed {
			return Ok(
				"Quest completed. Claim was sent, but reward confirmation is pending.".into(),
			);
		}
	}
	Ok("Quest completion confirmed by Discord.".into())
}
fn video_timestamp(initial: f64, elapsed: Duration, target: u32) -> f64 {
	(initial + elapsed.as_secs_f64()).min(f64::from(target))
}

#[cfg(test)]
mod tests {
	use super::*;
	struct Dropped(Option<tokio::sync::oneshot::Sender<()>>);
	impl Drop for Dropped {
		fn drop(&mut self) {
			if let Some(send) = self.0.take() {
				let _ = send.send(());
			}
		}
	}
	#[tokio::test]
	async fn account_replacement_and_voice_departure_abort_pending_jobs() {
		let secret =
			Arc::new(SessionSecret::from_owner_input("SYNTHETIC_QUEST_SESSION".into()).unwrap());
		let voice = VoiceContext {
			user: Id(1),
			channel: Id(2),
			guild: None,
		};
		let mut session = Session {
			secret: Some(secret),
			generation: 1,
			offline: false,
			proxy: watch::channel(Some(ApiProxy::Direct)).1,
			voice: Some(voice),
		};
		let mut view = Quests::default();
		view.observe_session(&session);
		for replace_account in [true, false] {
			let (started, ready) = tokio::sync::oneshot::channel();
			let (finished, released) = tokio::sync::oneshot::channel();
			let task = tokio::spawn(async move {
				let _drop = Dropped(Some(finished));
				let _ = started.send(());
				std::future::pending::<()>().await;
			});
			ready.await.unwrap();
			view.job = Some(Job {
				task,
				updates: mpsc::channel(1).1,
				voice: Some(voice),
			});
			if replace_account {
				session.generation += 1;
			} else {
				session.voice = None;
			}
			view.observe_session(&session);
			assert!(view.job.is_none());
			tokio::time::timeout(Duration::from_secs(2), released)
				.await
				.unwrap()
				.unwrap();
		}
	}
}
