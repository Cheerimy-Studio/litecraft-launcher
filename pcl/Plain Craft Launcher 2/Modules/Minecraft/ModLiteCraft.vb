Imports System.Diagnostics
Imports System.IO

''' <summary>
''' LiteCraft 26.3 的路径、版本注册、资源准备与游戏名校验。
''' </summary>
Public Module ModLiteCraft

    Public Function BaseDir() As String
        Return Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "LiteCraft")
    End Function

    Public Function BackendPath() As String
        Return Path.Combine(BaseDir(), "LiteCraft-Launther.exe")
    End Function

    Public Function GamePath() As String
        Return Path.Combine(BaseDir(), "litecraft.exe")
    End Function

    Public Function DataDir() As String
        Return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "LiteCraft")
    End Function

    Public Function ResourcesPath() As String
        Dim Bundled As String = Path.Combine(BaseDir(), "resources", "26.3")
        If File.Exists(Path.Combine(Bundled, "horizon-resources.json")) Then Return Bundled
        Return Path.Combine(DataDir(), "resources", "26.3")
    End Function

    Public Function GameDirPath() As String
        Return Path.Combine(DataDir(), "game")
    End Function

    Public Function HomePath() As String
        Return DataDir()
    End Function

    ''' <summary>
    ''' 游戏文件与资源是否已就绪。
    ''' </summary>
    Public Function Installed() As Boolean
        Return File.Exists(GamePath()) AndAlso File.Exists(Path.Combine(ResourcesPath(), "horizon-resources.json"))
    End Function

    ''' <summary>
    ''' 把 LiteCraft 26.3 注册到 PCL 的版本列表，供启动页直接启动。
    ''' </summary>
    Public Sub EnsureVersion()
        Try
            If String.IsNullOrEmpty(McFolderSelected) Then Return
            If Not Installed() Then Return

            Dim VersionName As String = "LiteCraft 26.3"
            Dim Folder As String = Path.Combine(McFolderSelected, "versions", VersionName)
            Dim JsonPath As String = Path.Combine(Folder, VersionName & ".json")
            Dim MarkerPath As String = Path.Combine(Folder, "litecraft.json")
            Dim Marker As New JObject()
            Marker("game") = GamePath()
            Marker("resources") = ResourcesPath()
            Marker("gameDir") = GameDirPath()
            Marker("home") = HomePath()

            If File.Exists(JsonPath) AndAlso File.Exists(MarkerPath) Then
                Try
                    Dim Old As JObject = JObject.Parse(File.ReadAllText(MarkerPath))
                    If Old("game")?.ToString() = GamePath() AndAlso Old("resources")?.ToString() = ResourcesPath() Then Return
                Catch
                End Try
            End If

            Directory.CreateDirectory(Folder)
            Dim SourceJson As String = Nothing
            For Each Candidate As String In {
                Path.Combine(DataDir(), "minecraft", "versions", "26.3", "26.3.json"),
                Path.Combine(BaseDir(), "minecraft", "versions", "26.3", "26.3.json"),
                Path.Combine(BaseDir(), "version", "26.3.json"),
                Path.Combine(McFolderSelected, "versions", "26.3", "26.3.json")
            }
                If File.Exists(Candidate) Then
                    SourceJson = File.ReadAllText(Candidate)
                    Exit For
                End If
            Next

            Dim Root As JObject
            If SourceJson IsNot Nothing Then
                Root = JObject.Parse(SourceJson)
            Else
                Root = New JObject()
                Root("id") = "26.3"
                Root("type") = "release"
                Root("mainClass") = "net.minecraft.client.main.Main"
                Root("releaseTime") = "2026-09-15T11:23:02+00:00"
                Root("assets") = "34"
                Root("assetIndex") = New JObject From {New JProperty("id", "34")}
                Root("downloads") = New JObject()
                Root("libraries") = New JArray()
                Root("arguments") = New JObject From {New JProperty("game", New JArray()), New JProperty("jvm", New JArray())}
            End If
            Root("id") = VersionName
            Root("litecraft") = Marker

            File.WriteAllText(JsonPath, Root.ToString())
            File.WriteAllText(MarkerPath, Marker.ToString())
            Try
                LoaderFolderRun(McInstanceListLoader, McFolderSelected, LoaderFolderRunType.ForceRun, MaxDepth:=1, ExtraPath:="versions\")
            Catch
            End Try
            Logger.Info("LiteCraft 26.3 已添加到版本列表")
        Catch ex As Exception
            Logger.Warn(ex, "添加 LiteCraft 版本失败")
        End Try
    End Sub

    ''' <summary>
    ''' 调用 Rust 后端下载并准备 26.3 资源。返回进程退出码。
    ''' </summary>
    Public Function Prepare() As Integer
        If Not File.Exists(BackendPath()) Then
            Logger.Warn("LiteCraft 后端不存在：" & BackendPath())
            Return -1
        End If
        Try
            Dim Info As New ProcessStartInfo(BackendPath(), "--prepare --source official") With {
                .WorkingDirectory = BaseDir(),
                .UseShellExecute = False,
                .RedirectStandardOutput = True,
                .RedirectStandardError = True,
                .CreateNoWindow = True,
                .StandardOutputEncoding = Encoding.UTF8,
                .StandardErrorEncoding = Encoding.UTF8
            }
            Dim P As New Process With {.StartInfo = Info}
            AddHandler P.OutputDataReceived, Sub(sender, e) If e.Data IsNot Nothing Then Logger.Info("[LiteCraft] " & e.Data)
            AddHandler P.ErrorDataReceived, Sub(sender, e) If e.Data IsNot Nothing Then Logger.Info("[LiteCraft] " & e.Data)
            P.Start()
            P.BeginOutputReadLine()
            P.BeginErrorReadLine()
            P.WaitForExit()
            Return P.ExitCode
        Catch ex As Exception
            Logger.Warn(ex, "LiteCraft 资源准备失败")
            Return -1
        End Try
    End Function

    Public Function RenderDistance() As Integer
        Dim Value As Integer = Settings.Get(Of Integer)("LiteCraftRenderDistance")
        If Value < 2 OrElse Value > 512 Then Value = 16
        Return Value
    End Function

End Module
