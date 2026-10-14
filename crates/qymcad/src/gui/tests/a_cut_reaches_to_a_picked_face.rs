//! A CUT THAT REACHES TO A PICKED FACE: only planar faces parallel to the sketch plane are taken (a
//! side wall of the plate is perpendicular and is passed over), the extent is the face's own place,
//! reopening brings the face back, and a face that no longer resolves makes the edit wait for a pick,
//! writing nothing.
#[cfg(test)]
mod tests {
    use super::super::hand::Hand;
    use super::super::App;
    use qymcad_core::feature::{Extent, FeatureKind, Stop};
    use qymcad_core::refs::Ref;

    /// A plate 60 x 40 x 12 with a 10 x 10 pocket sketch inside it, on the XY plane of the plate's base,
    /// and the cut command open over it with its extent set by a face. Returns the plate's body.
    fn plate_with_a_pocket_cut(hand: &mut Hand) -> qymcad_core::model::Id {
        assert!(hand.press_word(&qymcad_i18n::tr("plane-xy-table"), egui::pos2(0.0, 0.0)), "setup: a sketch is started on XY");
        hand.sk_tool(2).click2d(0.0, 0.0).click2d(60.0, 40.0);
        assert!(hand.press_word(&qymcad_i18n::tr("wb-finish"), egui::pos2(0.0, 0.0)), "setup: the sketch is finished");
        hand.close_window();
        hand.look_at([30.0, 20.0, 6.0], 8.0).tool(1).set("height", 12.0).enter();
        let plate = hand.app.project.timeline.iter().find(|n| matches!(n.kind, FeatureKind::Extrude { .. })).and_then(|n| n.kind.body()).expect("setup: the plate");
        assert!(hand.press_word(&qymcad_i18n::tr("plane-xy-table"), egui::pos2(0.0, 0.0)), "the pocket sketch is started on XY");
        hand.sk_tool(2).click2d(10.0, 10.0).click2d(20.0, 20.0);
        assert!(hand.press_word(&qymcad_i18n::tr("wb-finish"), egui::pos2(0.0, 0.0)), "the pocket sketch is finished");
        hand.close_window();
        hand.app.feat.op = 2; // the cut, as the Part hotkey takes it
        hand.tool(1);
        assert!(hand.press_word(&qymcad_i18n::tr("cmd-up-to-face"), egui::pos2(0.0, 0.0)), "the extent is switched to a face");
        assert_eq!(hand.app.tools.cmd.pick_face, Some(qymcad_ui_state::FaceSlot::Stop), "the stop face is asked for at once");
        plate
    }

    #[test]
    fn a_perpendicular_face_is_passed_over_and_a_parallel_one_is_taken() {
        let mut app = App::default();
        let mut hand = Hand::new(&mut app);
        plate_with_a_pocket_cut(&mut hand);
        // a SIDE wall of the plate: perpendicular to the sketch plane, so the pick passes it over
        hand.click([60.0, 20.0, 6.0]);
        assert!(hand.app.tools.cmd.stop_face.is_none(), "a perpendicular face was taken as the stop");
        assert_eq!(hand.app.status, qymcad_ui_state::Notice::refusal(qymcad_i18n::tr("vp-miss-parallel-face")), "the pick said why it took nothing");
        // the TOP face: parallel, and it is taken
        hand.click([30.0, 20.0, 12.0]);
        let taken = hand.app.tools.cmd.stop_face.clone().expect("the top face is taken as the stop");
        assert_ne!(taken.key.id, 0, "the stop carries the persistent face id");
        // the length arrow is off while the face drives the stop
        assert!(qymcad_ui_state::feat_cmd_arrows(&hand.app.tools.cmd, &hand.app.tools.gsel, &hand.app.project, hand.app.feat.flip).is_empty(), "an arrow is offered while a face sets the stop");
        hand.enter();
        crate::gui::io_jobs::regenerate_now(&mut hand.app.rebuild_ctx()); // the apply builds on a worker; the measure waits for it
        let (_, node) = hand.app.project.timeline.iter().enumerate().find(|(_, n)| matches!(n.kind, FeatureKind::Combine { .. })).map(|(i, n)| (i, n.id)).expect("the cut");
        assert!(!hand.app.project.regen_errors.contains_key(&node), "the cut builds: {:?}", hand.app.project.regen_errors);
        match &hand.app.project.timeline.iter().find(|n| n.id == node).expect("the cut").kind {
            FeatureKind::Combine { extent, .. } => assert!(matches!(extent.stop, Stop::UpToFace(_)), "the cut stops by a face"),
            _ => panic!("the node is not a combine"),
        }
        // 60 x 40 x 12 = 28800, the pocket takes the whole 10 x 10 column of 12 = 1200
        let consumed = qymcad_ui_state::consumed_bodies(&hand.app.project);
        let v: f64 = hand.app.live.shapes.iter().filter(|(b, _)| !consumed.contains(b)).map(|(_, s)| s.volume()).sum::<f64>();
        assert!((v - 27600.0).abs() < 5.0, "the cut reached exactly the top face: V={v:.1}");
    }

    #[test]
    fn taking_the_stop_by_a_face_leaves_the_flat_sheet_of_the_sketch() {
        // The command opens on the flat sheet, its contour step; the face pick lives in the 3D viewport,
        // where the body can be turned to reach a far side. Taking the mode must turn the view there, or
        // the button looks dead from the flat sheet - nothing on screen says where the click would land.
        let mut app = App::default();
        let mut hand = Hand::new(&mut app);
        assert!(hand.press_word(&qymcad_i18n::tr("plane-xy-table"), egui::pos2(0.0, 0.0)), "setup: a sketch is started on XY");
        hand.sk_tool(2).click2d(0.0, 0.0).click2d(60.0, 40.0);
        assert!(hand.press_word(&qymcad_i18n::tr("wb-finish"), egui::pos2(0.0, 0.0)), "setup: the sketch is finished");
        hand.close_window();
        hand.look_at([30.0, 20.0, 6.0], 8.0).tool(1).set("height", 12.0).enter(); // the plate
        hand.app.chosen.sel = super::super::Sel::Sketch(0); // its own sketch stands for the cut
        hand.app.feat.op = 2;
        hand.tool(1);
        hand.app.viewing.mode_3d = false; // the flat sheet, as the command can be taken from within it
        assert!(hand.press_word(&qymcad_i18n::tr("cmd-up-to-face"), egui::pos2(0.0, 0.0)), "the extent is switched to a face");
        assert!(hand.app.viewing.mode_3d, "the view turned to 3D, where the face is picked");
        // and the pick answers there: the top face of the plate is taken after a turn of the view
        hand.look_at([30.0, 20.0, 6.0], 8.0);
        hand.click([30.0, 20.0, 12.0]);
        assert!(hand.app.tools.cmd.stop_face.is_some(), "the face is taken in the 3D view the mode opened");
    }

    #[test]
    fn the_picked_face_comes_back_on_reopen_and_a_lost_one_waits() {
        let mut app = App::default();
        let mut hand = Hand::new(&mut app);
        plate_with_a_pocket_cut(&mut hand);
        hand.click([30.0, 20.0, 12.0]);
        hand.enter();
        crate::gui::io_jobs::regenerate_now(&mut hand.app.rebuild_ctx()); // the apply builds on a worker; the reopen waits for it
        let (ti, node) = hand.app.project.timeline.iter().enumerate().find(|(_, n)| matches!(n.kind, FeatureKind::Combine { .. })).map(|(i, n)| (i, n.id)).expect("the cut");
        // REOPEN: the stop face comes back picked
        let label = crate::gui::panels_tree::feature_row_label(&hand.app.project, ti);
        assert!(hand.double_click_word(&label), "the tree shows the cut as {label:?}");
        assert!(hand.app.tools.cmd.edit.is_some(), "the cut did not reopen; the program says {:?}", hand.app.status);
        assert!(hand.app.tools.cmd.stop_face.is_some(), "the reopened cut does not hold its face");
        assert!(qymcad_part::cmd_ready(&mut hand.app.part_ctx()), "the reopened cut is not ready to apply");
        hand.enter(); // applied unchanged
                      // LOST: a name no face of the plate carries, as after an edit above renamed it
        if let Some(n) = hand.app.project.timeline.iter_mut().find(|n| n.id == node) {
            if let FeatureKind::Combine { extent: Extent { stop: Stop::UpToFace(fr), .. }, .. } = &mut n.kind {
                fr.face = Ref::picks(&[0x6000_ffff]);
            }
            n.dirty = true;
        }
        qymcad_ui_state::regenerate_now(&mut hand.app.rebuild_ctx());
        assert!(hand.app.project.regen_errors.contains_key(&node), "setup: the cut with its face lost is red");
        assert!(hand.double_click_word(&label), "the red cut is shown as {label:?}");
        assert!(hand.app.tools.cmd.stop_face.is_none(), "GUARD: the cut reopened with a face picked: {:?}", hand.app.tools.cmd.stop_face);
        assert!(!qymcad_part::cmd_ready(&mut hand.app.part_ctx()), "the cut with no face picked is ready to apply");
        hand.enter();
        let still = hand.app.project.timeline.iter().find(|n| n.id == node).and_then(|n| match &n.kind {
            FeatureKind::Combine { extent: Extent { stop: Stop::UpToFace(fr), .. }, .. } => Some(fr.face.clone()),
            _ => None,
        });
        assert_eq!(still, Some(Ref::picks(&[0x6000_ffff])), "Enter with no face picked rewrote the reference");
    }
}
