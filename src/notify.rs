use anyhow::Result;
use jni::{
    objects::{JObject, JValue},
    JNIEnv, JavaVM,
};

const ID: i32 = 1;
const CHANNEL: &str = "session";
const POST_NOTIFICATIONS: &str = "android.permission.POST_NOTIFICATIONS";

pub fn request() {
    if let Err(err) = with(|env, activity, _| {
        let permission = JObject::from(env.new_string(POST_NOTIFICATIONS)?);
        let granted = env
            .call_method(
                activity,
                "checkSelfPermission",
                "(Ljava/lang/String;)I",
                &[JValue::Object(&permission)],
            )?
            .i()?;
        if granted == 0 {
            return Ok(());
        }
        let array = env.new_object_array(1, "java/lang/String", JObject::null())?;
        env.set_object_array_element(
            &array,
            0,
            JObject::from(env.new_string(POST_NOTIFICATIONS)?),
        )?;
        env.call_method(
            activity,
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[JValue::Object(&array), JValue::Int(7)],
        )?;
        Ok(())
    }) {
        dioxus::logger::tracing::error!("notification permission failed: {err:#}");
    }
}

pub fn show(title: &str, body: &str, started_at: i64, rest: Option<(i64, i64)>) {
    if let Err(err) = post(title, body, started_at, rest) {
        dioxus::logger::tracing::error!("notification failed: {err:#}");
    }
}

pub fn clear() {
    if let Err(err) = with(|env, activity, _| {
        let manager = manager(env, activity)?;
        env.call_method(&manager, "cancel", "(I)V", &[JValue::Int(ID)])?;
        Ok(())
    }) {
        dioxus::logger::tracing::error!("notification clear failed: {err:#}");
    }
}

fn post(title: &str, body: &str, started_at: i64, rest: Option<(i64, i64)>) -> Result<()> {
    with(|env, activity, sdk| {
        let manager = manager(env, activity)?;
        if sdk >= 26 {
            channel(env, &manager)?;
        }
        let builder = builder(env, activity, sdk)?;
        set_text(env, &builder, "setContentTitle", title)?;
        set_text(env, &builder, "setContentText", body)?;
        env.call_method(
            &builder,
            "setWhen",
            "(J)Landroid/app/Notification$Builder;",
            &[JValue::Long(started_at.saturating_mul(1000))],
        )?;
        flag(env, &builder, "setUsesChronometer", true)?;
        flag(env, &builder, "setOnlyAlertOnce", true)?;
        if let Some((remaining, total)) = rest {
            let total = total.max(1);
            let remaining = remaining.clamp(0, total);
            env.call_method(
                &builder,
                "setProgress",
                "(IIZ)Landroid/app/Notification$Builder;",
                &[
                    JValue::Int(int32(total)),
                    JValue::Int(int32(remaining)),
                    JValue::Bool(0),
                ],
            )?;
        }
        let icon = env.find_class("android/R$drawable")?;
        let icon = env.get_static_field(&icon, "stat_notify_sync", "I")?.i()?;
        env.call_method(
            &builder,
            "setSmallIcon",
            "(I)Landroid/app/Notification$Builder;",
            &[JValue::Int(icon)],
        )?;
        if let Ok(target) = launch_intent(env, activity) {
            if !target.is_null() {
                env.call_method(
                    &builder,
                    "setContentIntent",
                    "(Landroid/app/PendingIntent;)Landroid/app/Notification$Builder;",
                    &[JValue::Object(&target)],
                )?;
            }
        }
        let notification = env
            .call_method(&builder, "build", "()Landroid/app/Notification;", &[])?
            .l()?;
        env.call_method(
            &manager,
            "notify",
            "(ILandroid/app/Notification;)V",
            &[JValue::Int(ID), JValue::Object(&notification)],
        )?;
        Ok(())
    })
}

fn launch_intent<'local>(env: &mut JNIEnv<'local>, activity: &JObject) -> Result<JObject<'local>> {
    let package = env
        .call_method(activity, "getPackageName", "()Ljava/lang/String;", &[])?
        .l()?;
    let packages = env
        .call_method(
            activity,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )?
        .l()?;
    let intent = env
        .call_method(
            &packages,
            "getLaunchIntentForPackage",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&package)],
        )?
        .l()?;
    if intent.is_null() {
        return Ok(intent);
    }
    let flags = env.get_static_field("android/app/PendingIntent", "FLAG_IMMUTABLE", "I")?.i()?
        | env.get_static_field("android/app/PendingIntent", "FLAG_UPDATE_CURRENT", "I")?.i()?;
    Ok(env
        .call_static_method(
            "android/app/PendingIntent",
            "getActivity",
            "(Landroid/content/Context;ILandroid/content/Intent;I)Landroid/app/PendingIntent;",
            &[
                JValue::Object(activity),
                JValue::Int(0),
                JValue::Object(&intent),
                JValue::Int(flags),
            ],
        )?
        .l()?)
}

fn with<T>(call: impl FnOnce(&mut JNIEnv, &JObject, i32) -> Result<T>) -> Result<T> {
    let ctx = ndk_context::android_context();
    // SAFETY: tao initialises ndk-context with the live JVM and application Context before launch.
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    // SAFETY: same source as above; the reference is not kept past this function.
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
    let sdk = env
        .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
        .i()?;
    call(&mut env, &activity, sdk)
}

fn manager<'local>(env: &mut JNIEnv<'local>, activity: &JObject) -> Result<JObject<'local>> {
    let name = JObject::from(env.new_string("notification")?);
    Ok(env
        .call_method(
            activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&name)],
        )?
        .l()?)
}

fn channel(env: &mut JNIEnv, manager: &JObject) -> Result<()> {
    let id = JObject::from(env.new_string(CHANNEL)?);
    let label = JObject::from(env.new_string("Workout")?);
    let channel = env.new_object(
        "android/app/NotificationChannel",
        "(Ljava/lang/String;Ljava/lang/CharSequence;I)V",
        &[
            JValue::Object(&id),
            JValue::Object(&label),
            JValue::Int(2),
        ],
    )?;
    env.call_method(
        manager,
        "createNotificationChannel",
        "(Landroid/app/NotificationChannel;)V",
        &[JValue::Object(&channel)],
    )?;
    Ok(())
}

fn builder<'local>(
    env: &mut JNIEnv<'local>,
    activity: &JObject,
    sdk: i32,
) -> Result<JObject<'local>> {
    if sdk >= 26 {
        let channel = JObject::from(env.new_string(CHANNEL)?);
        Ok(env.new_object(
            "android/app/Notification$Builder",
            "(Landroid/content/Context;Ljava/lang/String;)V",
            &[JValue::Object(activity), JValue::Object(&channel)],
        )?)
    } else {
        let builder = env.new_object(
            "android/app/Notification$Builder",
            "(Landroid/content/Context;)V",
            &[JValue::Object(activity)],
        )?;
        env.call_method(
            &builder,
            "setPriority",
            "(I)Landroid/app/Notification$Builder;",
            &[JValue::Int(-1)],
        )?;
        Ok(builder)
    }
}

fn set_text(env: &mut JNIEnv, builder: &JObject, method: &str, value: &str) -> Result<()> {
    let value = JObject::from(env.new_string(value)?);
    env.call_method(
        builder,
        method,
        "(Ljava/lang/CharSequence;)Landroid/app/Notification$Builder;",
        &[JValue::Object(&value)],
    )?;
    Ok(())
}

fn flag(env: &mut JNIEnv, builder: &JObject, method: &str, value: bool) -> Result<()> {
    env.call_method(
        builder,
        method,
        "(Z)Landroid/app/Notification$Builder;",
        &[JValue::Bool(u8::from(value))],
    )?;
    Ok(())
}

fn int32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
